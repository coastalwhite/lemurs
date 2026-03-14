{ self, pkgs }:

pkgs.testers.nixosTest {
  name = "able-to-login";
  nodes.machine = { config, pkgs, ... }: {
    imports = [
    ];

    services.displayManager.enable = true;
    services.displayManager.lemurs = {
      enable = true;
      package = self.packages.${pkgs.system}.default;
    };
		virtualisation.qemu.options = [ "-vga none -device virtio-gpu-pci" ];
    services.seatd.enable = true;
    users.users.alice.extraGroups = [ "seat" ];
		users.users.alice = {
			isNormalUser = true;
			initialPassword = "test123";
		};

    system.stateVersion = "25.11";
  };

  testScript = ''
machine.start()
machine.wait_for_file("/var/log/lemurs.log", 60)
machine.sleep(3)
machine.screenshot("Debug1")

machine.send_key('down')
machine.send_chars('alice\n', 0.2)
machine.send_chars('test123\n', 0.2)

machine.sleep(3)
machine.send_chars("touch ~/test-file\n")
machine.wait_for_file("/home/alice/test-file", 10)
  '';
}