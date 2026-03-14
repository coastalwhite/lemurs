{
  description = "A flake to build a Rust project to WASM";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    utils.url = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      utils,
      rust-overlay,
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        packageName = "lemurs";

        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        # Use nixpkgs' built-in rustPlatform for sandboxed builds (e.g. NixOS tests)
        nixpkgsRustPlatform = pkgs.rustPlatform;
      in
      {
        packages.default = nixpkgsRustPlatform.buildRustPackage {
          name = packageName;
          src = ./.;

          postPatch = ''
            substituteInPlace extra/config.toml \
              --replace-fail "/usr/sh" "${pkgs.bash}/bin/bash"

            substituteInPlace extra/config.toml \
              --replace-fail "/usr/bin/X" "${pkgs.xorg.xorgserver}/bin/X"

            substituteInPlace extra/config.toml \
              --replace-fail "/usr/bin/xauth" "${pkgs.xorg.xauth}/bin/xauth"
          '';

          buildInputs = [
            pkgs.linux-pam
          ];

          meta.mainProgram = "lemurs";

          cargoLock.lockFile = ./Cargo.lock;
        };
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            linux-pam
            rustToolchain  # use overlay toolchain in dev shell
            cargo-dist
          ];
        };
				checks = {
					able-to-login = pkgs.callPackage ./tests/able-to-login.nix { inherit self; };
					graphical-sway-login = pkgs.callPackage ./tests/graphical-sway-login.nix { inherit self; };
					relogin = pkgs.callPackage ./tests/relogin.nix { inherit self; };
				};
      }
    );
}
