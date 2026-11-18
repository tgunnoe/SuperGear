# Contract for the manifest `hgpack list --json` prints: the structure of
# Heavy Gear's resource pack (docs/spec/heavy-gear.md §5), restated apart
# from the Rust reader so that each checks the other.
#
#   hgpack list --json <game-dir> > manifest.json
#   nickel export manifest.json --apply-contract contracts/pack.ncl > /dev/null
#
# The manifest lists game data and is never committed (AGENTS.md I1).

let Nat = std.number.Nat in

let Manifest = {
  pack_size | Nat,
  reserved | Nat,
  codecs | Array String,
  types | Array { type | String, count | Nat },
  entries
    | Array {
      name | String,
      type | String,
      offset | Nat,
      method | Nat,
      unpacked | Nat,
      packed | Nat,
      attr | String,
    },
}
in

# Bytes in a record header: tag[12], method u16, unpacked u32, packed u32.
let header = 22 in

let key = fun e => "%{e.name}.%{e.type}" in

# The first non-null element of `xs`, or null. Elements are lazy, so the
# ones after it are never evaluated.
let first_of = fun xs =>
  std.array.fold_left (fun found x => if found != null then found else x) null xs
in

# The message of the first check that fails, or null.
let first_failure = fun checks =>
  first_of (std.array.map (fun c => if c.ok then null else c.message) checks)
in

# The records tile the pack in manifest order: each starts where the
# previous one ends, the first at 0, the last ending at the pack's size.
let tiling = fun m =>
  let walk =
    std.array.fold_left
      (fun acc e =>
        if acc.error != null then
          acc
        else if e.offset != acc.end then
          { end = acc.end, error = "%{key e} starts at %{std.to_string e.offset}, the previous record ends at %{std.to_string acc.end}" }
        else
          { end = e.offset + header + e.packed, error = null }
      )
      { end = 0, error = null }
      m.entries
  in
  if walk.error != null then
    walk.error
  else if walk.end != m.pack_size then
    "records end at %{std.to_string walk.end}, the pack is %{std.to_string m.pack_size} bytes"
  else
    null
in

# Method 0 is stored (unpacked = packed); method k names codec k - 1.
let methods = fun m =>
  let codecs = std.array.length m.codecs in
  first_failure (
    std.array.flat_map
      (fun e => [
        {
          ok = e.method <= codecs,
          message = "%{key e}: method %{std.to_string e.method}, but the codec table has %{std.to_string codecs} entries",
        },
        {
          ok = e.method != 0 || e.unpacked == e.packed,
          message = "%{key e}: stored, yet unpacked %{std.to_string e.unpacked} differs from packed %{std.to_string e.packed}",
        },
      ])
      m.entries
  )
in

# The type table's counts sum to the entry count and equal each type's
# tally among the entries, and no entry has a type outside the table.
let types = fun m =>
  let tally =
    std.array.fold_left
      (fun acc e =>
        if std.record.has_field e.type acc then
          std.record.update e.type (std.record.get e.type acc + 1) acc
        else
          std.record.insert e.type 1 acc
      )
      {}
      m.entries
  in
  let sum = std.array.fold_left (fun s t => s + t.count) 0 m.types in
  let table = std.array.map (fun t => t.type) m.types in
  let n = std.array.length m.entries in
  first_failure (
    [
      {
        ok = sum == n,
        message = "type counts sum to %{std.to_string sum}, the index has %{std.to_string n} entries",
      }
    ]
    @ std.array.map
      (fun t =>
        let seen = if std.record.has_field t.type tally then std.record.get t.type tally else 0 in
        {
          ok = seen == t.count,
          message = "type %{t.type}: the table says %{std.to_string t.count}, the entries have %{std.to_string seen}",
        }
      )
      m.types
    @ std.array.map
      (fun t => { ok = std.array.elem t table, message = "type %{t} is not in the type table" })
      (std.record.fields tally)
  )
in

std.contract.custom (fun label value =>
  let m = std.contract.apply Manifest label value in
  std.deep_seq m (
    let failure = first_of [types m, methods m, tiling m] in
    if failure == null then 'Ok m else 'Error { message = failure }
  )
)
