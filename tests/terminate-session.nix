{ self, pkgs }:

pkgs.testers.nixosTest {
  name = "terminate-session";
  nodes.machine =
    { config, pkgs, ... }:
    let
      # On first login: record the session ID and terminate it via loginctl.
      # This simulates the user (or a compositor crash handler) calling
      # `loginctl terminate-session` from within the session.  Lemurs should
      # survive and return to the login screen.
      #
      # On second login: just touch a file so the test can assert lemurs
      # returned to the login screen successfully.
      aliceSession = pkgs.writeShellScript "alice-session" ''
        if [ ! -f /tmp/alice-login1-done ]; then
          touch /tmp/alice-login1-done
          loginctl terminate-session "$XDG_SESSION_ID"
        else
          touch /tmp/alice-login2-done
          ${pkgs.sway}/bin/swaymsg exit
        fi
      '';
    in
    {
      services.displayManager.enable = true;
      services.displayManager.lemurs = {
        enable = true;
        package = self.packages.${pkgs.system}.default;
      };

      virtualisation.qemu.options = [ "-vga none -device virtio-gpu-pci" ];
      services.seatd.enable = true;
      programs.sway.enable = true;

      users.users.alice = {
        isNormalUser = true;
        initialPassword = "test123";
        extraGroups = [ "seat" "video" "input" ];
      };

      systemd.tmpfiles.rules =
        let
          swayConfig = pkgs.writeText "alice-sway-config" ''
            exec ${pkgs.foot}/bin/foot -- ${aliceSession}
          '';
        in
        [
          "d /home/alice/.config 0755 alice users -"
          "d /home/alice/.config/sway 0755 alice users -"
          "L+ /home/alice/.config/sway/config - alice users - ${swayConfig}"
        ];

      system.stateVersion = "25.11";
    };

  testScript = ''
    machine.start()
    machine.wait_for_file("/var/log/lemurs.log", 60)
    machine.sleep(3)
    machine.screenshot("01_before_login")

    # --- First login: session is terminated via loginctl from inside ---
    machine.send_key('down')
    machine.send_chars('alice\n', 0.2)
    machine.send_chars('test123\n', 0.2)
    machine.wait_for_file("/tmp/alice-login1-done", 60)
    machine.screenshot("02_session_terminated_via_loginctl")

    # Lemurs must survive and return to the login screen.
    # Wait for the login screen to reappear, then log in again.
    machine.sleep(5)
    machine.screenshot("03_back_at_login_screen")

    # --- Second login: verifies lemurs is still alive ---
    machine.send_key('down')
    machine.send_chars('alice\n', 0.2)
    machine.send_chars('test123\n', 0.2)
    machine.wait_for_file("/tmp/alice-login2-done", 60)
    machine.screenshot("04_second_login_succeeded")
  '';
}
