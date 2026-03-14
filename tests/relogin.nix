{ self, pkgs }:

pkgs.testers.nixosTest {
  name = "relogin";
  nodes.machine =
    { config, pkgs, ... }:
    let
      aliceSession = pkgs.writeShellScript "alice-session" ''
        if [ ! -f /tmp/alice-login1-done ]; then
          touch /tmp/alice-login1-done
        else
          touch /tmp/alice-login2-done
        fi
        ${pkgs.sway}/bin/swaymsg exit
      '';
      bobSession = pkgs.writeShellScript "bob-session" ''
        touch /tmp/bob-login1-done
        ${pkgs.sway}/bin/swaymsg exit
      '';
    in
    {
      imports = [ ];

      services.displayManager.enable = true;
      services.displayManager.lemurs.enable = true;
      services.displayManager.lemurs.package = self.packages.${pkgs.system}.default;

      virtualisation.qemu.options = [ "-vga none -device virtio-gpu-pci" ];
      services.seatd.enable = true;
      programs.sway.enable = true;

      users.users.alice = {
        isNormalUser = true;
        initialPassword = "test123";
        extraGroups = [ "seat" "video" "input" ];
      };

      users.users.bob = {
        isNormalUser = true;
        initialPassword = "test456";
        extraGroups = [ "seat" "video" "input" ];
      };

      # Alice's sway config: first login touches login1-done, second touches login2-done.
      # We accomplish this by checking which marker already exists at startup.
      systemd.tmpfiles.rules =
        let
          aliceSwayConfig = pkgs.writeText "alice-sway-config" ''
            exec ${pkgs.foot}/bin/foot -- ${aliceSession}
          '';
          bobSwayConfig = pkgs.writeText "bob-sway-config" ''
            exec ${pkgs.foot}/bin/foot -- ${bobSession}
          '';
        in
        [
          "d /home/alice/.config 0755 alice users -"
          "d /home/alice/.config/sway 0755 alice users -"
          "L+ /home/alice/.config/sway/config - alice users - ${aliceSwayConfig}"
          "d /home/bob/.config 0755 bob users -"
          "d /home/bob/.config/sway 0755 bob users -"
          "L+ /home/bob/.config/sway/config - bob users - ${bobSwayConfig}"
        ];

      system.stateVersion = "25.11";
    };

  testScript = ''
    machine.start()
    machine.wait_for_file("/var/log/lemurs.log", 60)
    machine.sleep(3)
    machine.screenshot("01_before_first_login")

    # --- First login: alice ---
    machine.send_key('down')
    machine.send_chars('alice\n', 0.2)
    machine.send_chars('test123\n', 0.2)
    machine.wait_for_file("/tmp/alice-login1-done", 60)
    machine.screenshot("02_after_alice_first_session")

    # Wait for lemurs to return to the login screen
    machine.sleep(5)
    machine.screenshot("03_back_at_login_screen")

    # --- Second login: alice again (same user re-login) ---
    # Username is cached, so just focus it and confirm
    machine.send_key('down')
    machine.send_chars('alice\n', 0.2)
    machine.send_chars('test123\n', 0.2)
    machine.wait_for_file("/tmp/alice-login2-done", 60)
    machine.screenshot("04_after_alice_second_session")

    machine.sleep(5)
    machine.screenshot("05_back_at_login_screen_2")

    # --- Third login: bob (different user re-login) ---
    # lemurs cached alice's username, so focus lands on password.
    # Go up to username field, clear it, type bob, then enter password.
    machine.send_key('up')
    for _ in range(10):
        machine.send_key('backspace')
    machine.send_chars('bob\n', 0.2)
    machine.send_chars('test456\n', 0.2)
    machine.wait_for_file("/tmp/bob-login1-done", 60)
    machine.screenshot("06_after_bob_session")
  '';
}
