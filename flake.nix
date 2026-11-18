{
  description = "SuperGear — reverse engineering Heavy Gear (Activision, 1997)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # The pack reader and its command. Building it runs the test suite,
      # contract tests included (they call nickel). Tests that need the
      # installed game are #[ignore]d and do not run here.
      buildHgpack = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "hgpack";
        version = "0.1.0";
        src = pkgs.lib.fileset.toSource {
          root = ./.;
          fileset = pkgs.lib.fileset.unions [ ./Cargo.toml ./Cargo.lock ./crates ./contracts ];
        };
        cargoLock.lockFile = ./Cargo.lock;
        nativeCheckInputs = [ pkgs.nickel ];
      };

      # clippy with warnings as errors, and rustfmt, over the same sources.
      lint = pkgs: (buildHgpack pkgs).overrideAttrs (old: {
        pname = "hgpack-lint";
        nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy pkgs.rustfmt ];
        buildPhase = ''
          cargo fmt --all -- --check
          cargo clippy --offline --all-targets -- -D warnings
        '';
        doCheck = false;
        installPhase = "touch $out";
      });
    in
    {
      packages = forAll (pkgs: let hgpack = buildHgpack pkgs; in {
        inherit hgpack;
        default = hgpack;
      });

      checks = forAll (pkgs: {
        hgpack = buildHgpack pkgs;
        hgpack-lint = lint pkgs;
      });

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
