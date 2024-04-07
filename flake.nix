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
        packages.lemurs = rustPlatform.buildRustPackage {
          name = packageName;
          src = ./.;

          postPatch = ''
            substituteInPlace ./extra/config.toml --replace "/usr/sh" "${pkgs.bash}/bin/bash"
            substituteInPlace ./extra/config.toml --replace "/usr/bin/X" "${pkgs.xorg.xorgserver}/bin/X"
            substituteInPlace ./extra/config.toml --replace "/usr/bin/xauth" "${pkgs.xorg.xauth}/bin/xauth"
          '';

          buildInputs = [
            pkgs.linux-pam
          ];
          
          cargoHash = "sha256-rJLHfedg4y8cZH77AEA4AjE0TvWf9tdSjKiHZfvW+gw=";  
        };
        packages.default = packages.lemurs;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            linux-pam
          ];
        };
      }
  ) // {
    nixosModules.default = {
      pkgs,
      lib,
      config,
      ...
    }: let
      sessionData = config.services.xserver.displayManager.sessionData;
    in {
      options.services.lemurs = rec {
        enable = lib.mkEnableOption "Enable the Lemurs Display Manager";

        x11.enable = lib.mkEnableOption "Enable the X11 part of the Lemurs Display Manager";
        wayland.enable = lib.mkEnableOption "Enable the Wayland part of the Lemurs Display Manager";
        
        tty = lib.mkOption {
          type = lib.types.str;
          default = "tty2";
        };


        settings = {
          x11 = {
            xauth = lib.mkOption {
              type = lib.types.nullOr lib.types.package;
              default = if x11.enable then pkgs.xorg.xauth else null;
            };

            xorgserver = lib.mkOption {
              type = lib.types.nullOr lib.types.package;
              default = if x11.enable then pkgs.xorg.xorgserver else null;
            };
            
            xsessions = lib.mkOption {
              type = lib.types.path;
              default = "${sessionData}/share/xsessions";
            };
          };

          wayland = {
            wayland-sessions = lib.mkOption {
              type = lib.types.path;
              default = "${sessionData}/share/wayland-sessions";
            };
          };
        };
      };

      config = let
        cfg = config.services.lemurs;
      in lib.mkIf cfg.enable {
        nixpkgs.overlays = [
          (final: prev: { lemurs = self.packages.x86_64-linux.default; })
        ];

        security.pam.services.lemurs = {
          allowNullPassword = true;
          startSession = true;
          setLoginUid = false;
          enableGnomeKeyring = lib.mkDefault config.services.gnome.gnome-keyring.enable;
        };

        systemd.services."autovt@${cfg.tty}".enable = false;

        systemd.services.lemurs = {
          aliases = [ "display-manager.service" ];
          
          unitConfig = {
            Wants = [
              "systemd-user-sessions.service"
            ];

            After = [
              "systemd-user-sessions.service"
              "plymouth-quit-wait.service"
              "getty@${cfg.tty}.service"
            ];

            Conflicts = [
              "getty@${cfg.tty}.service"
            ];
          };

          serviceConfig = {
            ExecStart = ''
              ${pkgs.lemurs}/bin/lemurs                      \
                --xsessions  ${cfg.x11.xsessions}            \
                --wlsessions ${cfg.wayland.wayland-sessions}
            '';

            StandardInput = "tty";
            TTYPath = "/dev/${cfg.tty}";
            TTYReset = "yes";
            TTYVHangup = "yes";

            Type = "idle";
          };

          restartIfChanged = false;

          wantedBy = [ "graphical.target" ];
        };

        systemd.defaultUnit = "graphical.target";

      };
    };
  };
}
