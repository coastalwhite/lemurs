{ self, pkgs }:

pkgs.testers.nixosTest {
  name = "graphical-sway-login";
  nodes.machine =
    { config, pkgs, ... }:
    {
      imports = [ ];

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
        extraGroups = [
          "seat"
          "video"
          "input"
        ];
      };

      # Place a sway config in alice's home that opens foot, touches a file, then exits
      systemd.tmpfiles.rules =
        let
          swayConfig = pkgs.writeText "alice-sway-config" ''
            exec ${pkgs.foot}/bin/foot -- ${pkgs.bash}/bin/bash -c \
              'touch /tmp/sway-test-file; ${pkgs.sway}/bin/swaymsg exit'
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
    machine.screenshot("before_login")

    # Navigate to the Sway Test session in the environment switcher,
    # then enter credentials
    machine.send_key('down')
    machine.send_chars('alice\n', 0.2)
    machine.send_chars('test123\n', 0.2)

    # Wait for sway to start, run foot, touch the file, and exit
    machine.wait_for_file("/tmp/sway-test-file", 30)
    machine.screenshot("after_sway")
  '';
}
