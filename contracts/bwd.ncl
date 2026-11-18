# Contract for the dump `hgbwd dump --json` prints: the structure of one
# of Heavy Gear's BWD files (docs/spec/heavy-gear.md §5, "BWD files"),
# restated apart from the Rust reader so that each checks the other.
#
#   hgbwd dump --json M33_SCN1.BWD > dump.json
#   nickel export dump.json --apply-contract contracts/bwd.ncl > /dev/null
#
# It is stricter than the reader in one place: like every shipped file,
# the dump must begin with REV and DTBL. The dump holds game data and is
# never committed (AGENTS.md I1).

let Nat = std.number.Nat in

let Dump = {
  size | Nat,
  largest | Nat,
  unclosed_btags | Array Nat,
  chunks
    | Array {
      tag | String,
      offset | Nat,
      size | Nat,
      data | String,
      ..
    },
}
in

# Bytes of the file header (magic, size, largest chunk) and of a chunk's
# own header (tag, size).
let header = 12 in
let chunk_header = 8 in

# The engine's chunk tags, in the order of the table in HGSHELL.DLL and
# HGSIM.DLL.
let tags = [
  "REV", "DTBL", "PLNT", "PALG", "TERR", "CLIM", "LITE", "VWSP", "VWST", "INIT",
  "STBL", "MTBL", "GTBL", "BMPJ", "BMID", "BSEC", "BMEN", "FPRJ", "HRZM", "GNDM",
  "SKYM", "PLGO", "BLKX", "REPR", "ENDR", "BLK", "ELSB", "ENDB", "OBJ", "ANIM",
  "SCRG", "THNG", "GP", "CPTF", "PITF", "VPTF", "HUDF", "MGDF", "EYEO", "GT",
  "OBJL", "BTHG", "XPLO", "NAVP", "NAVO", "LITO", "TSK", "POS", "ROT", "INCL",
  "GRP", "GPS", "MOFF", "MON", "GON", "GOFF", "ANM2", "STAR", "VIEW", "POFO",
  "AFFL", "ORDR", "MUSI", "ASND", "LTBL", "PTBL", "FTBL", "HTXT", "PDSC", "SDSC",
  "SUPS", "CPGN", "PILT", "TECH", "NEWS", "CTNT", "DSSR", "MHLT", "WNLT", "THLT",
  "SMHL", "SWNL", "PTLT", "SLST", "FLST", "WTHR", "AMMO", "MLIT", "CAMO", "EQUP",
  "MLUT", "TLUT", "BTAG", "ETAG", "HGXP", "HGBT",
]
in

# Chunk sizes, header included, of the tags whose layout the reader
# decodes.
let fixed = {
  REV = 12,
  INCL = 20,
  MUSI = 24,
  TLUT = 20,
  MLUT = 20,
  BTAG = 24,
  ETAG = 12,
  MON = 8,
  MOFF = 8,
  ENDB = 8,
}
in

let at = fun c => "%{c.tag} at %{std.to_string c.offset}" in

let list = fun xs => "[%{std.string.join ", " (std.array.map std.to_string xs)}]" in

# The first non-null element of `xs`, or null. Elements are lazy, so the
# ones after it are never evaluated.
let first_of = fun xs =>
  std.array.fold_left (fun found x => if found != null then found else x) null xs
in

# The message of the first check that fails, or null.
let first_failure = fun checks =>
  first_of (std.array.map (fun c => if c.ok then null else c.message) checks)
in

# The chunks tile the file: the first starts after the header, each
# starts where the previous one ends, and the last ends at the file's size.
let tiling = fun d =>
  let walk =
    std.array.fold_left
      (fun acc c =>
        if acc.error != null then
          acc
        else if c.offset != acc.end then
          { end = acc.end, error = "%{at c}: the previous chunk ends at %{std.to_string acc.end}" }
        else if c.size < chunk_header then
          { end = acc.end, error = "%{at c}: size %{std.to_string c.size} is below its own header" }
        else
          { end = c.offset + c.size, error = null }
      )
      { end = header, error = null }
      d.chunks
  in
  if walk.error != null then
    walk.error
  else if walk.end != d.size then
    "chunks end at %{std.to_string walk.end}, the file is %{std.to_string d.size} bytes"
  else
    null
in

# Each chunk has a known tag, as many data bytes as its size says, and,
# for a decoded tag, its layout's size.
let chunks = fun d =>
  first_failure (
    std.array.flat_map
      (fun c => [
        {
          ok = std.array.elem c.tag tags,
          message = "%{at c}: tag %{c.tag} is not in the engine's table",
        },
        {
          ok = std.string.length c.data == 2 * (c.size - chunk_header),
          message = "%{at c}: size %{std.to_string c.size} with %{std.to_string (std.string.length c.data / 2)} data bytes",
        },
        {
          ok = !(std.record.has_field c.tag fixed) || std.record.get c.tag fixed == c.size,
          message = "%{at c}: %{std.to_string c.size} bytes, its layout has %{std.to_string (std.record.get c.tag fixed)}",
        },
      ])
      d.chunks
  )
in

# The header names the size of the largest chunk (0 for none).
let largest = fun d =>
  let actual = std.array.fold_left (fun m c => if c.size > m then c.size else m) 0 d.chunks in
  if actual == d.largest then
    null
  else
    "the header's largest chunk is %{std.to_string d.largest} bytes, the largest is %{std.to_string actual}"
in

# Every shipped file begins REV, DTBL.
let opening = fun d =>
  let first = std.array.map (fun c => c.tag) (std.array.slice 0 (std.number.min 2 (std.array.length d.chunks)) d.chunks) in
  if first == ["REV", "DTBL"] then
    null
  else
    "the file begins %{std.string.join " " first}, not REV DTBL"
in

# MON/MOFF and BLK/ENDB nest like parentheses.
let nesting = fun d =>
  let opener = { MOFF = "MON", ENDB = "BLK" } in
  let walk =
    std.array.fold_left
      (fun acc c =>
        if acc.error != null then
          acc
        else if c.tag == "MON" || c.tag == "BLK" then
          { stack = [c] @ acc.stack, error = null }
        else if std.record.has_field c.tag opener then
          if std.array.length acc.stack > 0 && (std.array.first acc.stack).tag == std.record.get c.tag opener then
            { stack = std.array.drop_first acc.stack, error = null }
          else
            { stack = acc.stack, error = "%{at c} closes nothing" }
        else
          acc
      )
      { stack = [], error = null }
      d.chunks
  in
  if walk.error != null then
    walk.error
  else if std.array.length walk.stack > 0 then
    "%{at (std.array.first walk.stack)} is never closed"
  else
    null
in

# Each ETAG closes an open BTAG, none is open at the end, and the BTAGs
# that a later BTAG reopened are the ones the dump lists.
let groups = fun d =>
  let walk =
    std.array.fold_left
      (fun acc c =>
        let next = acc.i + 1 in
        if acc.error != null then
          acc
        else if c.tag == "BTAG" then
          {
            i = next,
            open = acc.i,
            reopened = acc.reopened @ (if acc.open == null then [] else [acc.open]),
            error = null,
          }
        else if c.tag == "ETAG" && acc.open == null then
          { i = next, open = null, reopened = acc.reopened, error = "%{at c} closes nothing" }
        else if c.tag == "ETAG" then
          { i = next, open = null, reopened = acc.reopened, error = null }
        else
          { i = next, open = acc.open, reopened = acc.reopened, error = null }
      )
      { i = 0, open = null, reopened = [], error = null }
      d.chunks
  in
  if walk.error != null then
    walk.error
  else if walk.open != null then
    "the BTAG at chunk %{std.to_string walk.open} is never closed"
  else if walk.reopened != d.unclosed_btags then
    "the dump lists reopened BTAGs %{list d.unclosed_btags}, the chunks reopen %{list walk.reopened}"
  else
    null
in

std.contract.custom (fun label value =>
  let d = std.contract.apply Dump label value in
  std.deep_seq d (
    let failure = first_of [tiling d, chunks d, largest d, opening d, nesting d, groups d] in
    if failure == null then 'Ok d else 'Error { message = failure }
  )
)
