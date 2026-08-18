{ self, pkgs }:

pkgs.testers.nixosTest {
  name = "suspend-resume";
  nodes.machine =
    { config, pkgs, ... }:
    let
      # Alice logs into a Wayland session that stays up until the test asks it to
      # exit (by creating /tmp/please-exit). The clean exit is what makes lemurs
      # run `stop_logging()` and join the log-forwarding thread -- which is where
      # an interrupted poll surfaces as a spurious "Failed to wait for client".
      aliceSession = pkgs.writeShellScript "alice-session" ''
        touch /tmp/alice-session-started
        while [ ! -e /tmp/please-exit ]; do
          sleep 0.5
        done
        ${pkgs.sway}/bin/swaymsg exit
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

  # Regression test for the suspend/resume session teardown.
  #
  # The log-forwarding thread spawned per session blocks in `mio::Poll::poll`
  # (epoll_wait). During a real suspend/hibernate the kernel process freezer
  # interrupts that syscall with EINTR; mio 0.8 surfaces it as
  # ErrorKind::Interrupted rather than retrying. Without the fix the thread
  # returns that error and dies; `stop_logging()` -- which lemurs only runs once
  # the client has exited -- then joins the dead thread and propagates the error
  # out of the client `wait()`, logging "Failed to wait for client" and tearing
  # the session down on the next resume/logout.
  #
  # We reproduce the interrupt deterministically with the cgroup v2 freezer (the
  # same mechanism suspend uses): freezing then thawing the session's cgroup
  # delivers EINTR to the poll. We then end the session cleanly so `stop_logging`
  # runs. With the fix the poll is retried, the thread stays healthy, and the
  # session tears down cleanly with no error in the log.
  testScript = ''
    machine.start()
    machine.wait_for_file("/var/log/lemurs.log", 60)
    machine.sleep(3)
    machine.screenshot("01_before_login")

    # --- Log in and wait for the session to come up ---
    machine.send_key('down')
    machine.send_chars('alice\n', 0.2)
    machine.send_chars('test123\n', 0.2)
    machine.wait_for_file("/tmp/alice-session-started", 60)
    # Let the compositor settle so the log thread is parked in its blocking poll.
    machine.sleep(5)
    machine.screenshot("02_session_running")

    # The log thread runs in the session-leader child, which elogind placed in
    # the user's session scope alongside sway. Freezing that cgroup is what the
    # suspend freezer does to the whole system.
    sway_pid = machine.succeed("pgrep -x sway | head -n1").strip()
    cgroup = machine.succeed(
        "cut -d: -f3 /proc/%s/cgroup | head -n1" % sway_pid
    ).strip()
    freeze = "/sys/fs/cgroup%s/cgroup.freeze" % cgroup
    machine.succeed("test -e " + freeze)

    # --- Suspend surrogate: freeze then thaw, delivering EINTR to the poll ---
    machine.succeed("echo 1 > " + freeze)
    machine.sleep(2)
    machine.succeed("echo 0 > " + freeze)
    machine.sleep(3)
    machine.screenshot("03_after_resume")

    # --- End the session cleanly so lemurs runs stop_logging()/joins the thread ---
    machine.succeed("touch /tmp/please-exit")
    # Wait for lemurs to reap the client and return to the login screen.
    machine.sleep(8)
    machine.screenshot("04_after_session_exit")

    # The interrupted poll must not surface as a client wait failure. This line
    # appears with the bug and is absent with the fix.
    machine.fail("grep -q 'Failed to wait for client' /var/log/lemurs.log")
  '';
}
