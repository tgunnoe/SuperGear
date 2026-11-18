{
  description = "SuperGear — reverse engineering Heavy Gear (Activision, 1997)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            # Rust is the implementation language; Nickel carries contracts
            # and configuration. Both are pinned by this flake, not by rustup.
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
            nickel
            # Disc and archive handling for the original game media.
            p7zip
            python3
            # Headless verification of the Wine-hosted original (see
            # scripts/wine-smoke.sh): a virtual X server and a screenshot tool.
            xvfb
            xwininfo
            imagemagick
          ]
          # The original game is an i386 Windows binary. nixpkgs' 32-bit
          # capable Wine exists only on x86_64; on aarch64 running it needs an
          # x86 emulation layer (FEX/box64 + Hangover), which is tracked as an
          # unknown in AGENTS.md rather than assumed here.
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isx86_64 [ pkgs.wine pkgs.winetricks ];
        };
      });
    };
}
