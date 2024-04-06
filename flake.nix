{
  description = "A flake to build a Rust project to WASM";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    utils.url = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, utils, rust-overlay }:
    utils.lib.eachDefaultSystem (system:
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
      in rec {
        packages.default = rustPlatform.buildRustPackage {
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
          
          cargoLock.lockFile = ./Cargo.lock;
        };
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            linux-pam
          ];
        };

				nixosModules.default = {
					pkgs,
					lib,
					config,
					...
				}: {
					options.services.xserver.displayManager.lemurs = {
						enable = lib.mkEnableOption "Enable the Lemurs Display Manager";
					};

					config = let
						lemursConfig = config.services.xserver.displayManager.lemurs;
            sessionData = config.services.xserver.displayManager.sessionData;
					in lib.mkIf lemursConfig.enable {
						environment.systemPackages = [
							packages.default
						];

            services.xserver.displayManager.job = {
              environment = {
                exec = ''
                  exec ${packages.default}/bin/lemurs                  \
                    --xsessions  ${sessionData}/share/xsessions        \
                    --wlsessions ${sessionData}/share/wayland-sessions
                '';
              };
            };
					};
				};
      }
  );
}