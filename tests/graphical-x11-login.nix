{ self, pkgs }:

pkgs.testers.nixosTest {
  name = "graphical-x11-login";
  nodes.machine =
    { pkgs, ... }:
    {
      services.displayManager.enable = true;
      services.displayManager.lemurs = {
        enable = true;
        package = self.packages.${pkgs.system}.default;
        # Use :0 so wait_for_x() can find /tmp/.X11-unix/X0
        settings.x11.x11_display = ":0";
      };

      services.xserver.enable = true;
      services.xserver.windowManager.twm.enable = true;

      fonts.packages = with pkgs; [
        xorg.fontmiscmisc
        xorg.fontadobe75dpi
        xorg.fontalias
      ];

      virtualisation.qemu.options = [ "-vga none -device virtio-gpu-pci" ];

      users.users.alice = {
        isNormalUser = true;
        initialPassword = "test123";
        extraGroups = [ "video" "input" ];
      };

      systemd.tmpfiles.rules = [
        "L+ /home/alice/.xsession - alice users - ${pkgs.writeScript "xsession" ''
          #!${pkgs.bash}/bin/bash
          exec ${pkgs.xterm}/bin/xterm -fn fixed
        ''}"
      ];

      system.stateVersion = "25.11";
    };

  testScript = ''
    machine.start()
    machine.wait_for_file("/var/log/lemurs.log", 60)
    machine.sleep(3)
    machine.screenshot("01_before_login")

    # Navigate to the twm session and log in.
    machine.send_key('down')
    machine.send_chars('alice\n', 0.2)
    machine.send_chars('test123\n', 0.2)

    # Wait for the X server to come up.  xterm is the session process so it
    # will have focus immediately.
    machine.wait_for_x(20)
    machine.sleep(3)
    machine.screenshot("02_xterm_open")

    # Focus xterm and type into it using xdotool (XTEST extension).  This
    # tests that X11 application input works end-to-end through lemurs.
    machine.succeed(
        "DISPLAY=:0 XAUTHORITY=/home/alice/.Xauthority"
        " ${pkgs.xdotool}/bin/xdotool search --sync --onlyvisible --class XTerm windowfocus --sync"
    )
    machine.succeed(
        "DISPLAY=:0 XAUTHORITY=/home/alice/.Xauthority"
        " ${pkgs.xdotool}/bin/xdotool type --clearmodifiers 'touch /tmp/x11-keyboard-works'"
    )
    machine.succeed(
        "DISPLAY=:0 XAUTHORITY=/home/alice/.Xauthority"
        " ${pkgs.xdotool}/bin/xdotool key Return"
    )
    machine.wait_for_file("/tmp/x11-keyboard-works", 15)
    machine.screenshot("03_keyboard_input_works")
  '';
}
