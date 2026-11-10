import os
import pty
import select
import sys
import time

def main():
    smoke_dir = "/home/jian/playground/cave_smoke"
    cave_bin = "/home/jian/playground/cave/target/release/cave"
    main_dart = os.path.join(smoke_dir, "lib", "main.dart")

    master, slave = pty.openpty()
    pid = os.fork()

    if pid == 0:
        os.close(master)
        os.setsid()
        os.dup2(slave, 0)
        os.dup2(slave, 1)
        os.dup2(slave, 2)
        os.close(slave)
        os.chdir(smoke_dir)
        cmd = [cave_bin, "--with", "flutter", "flutter", "run", "-d", "cbe5535f"]
        os.execv(cave_bin, cmd)
    else:
        os.close(slave)
        output = []
        started = False
        reloaded = False
        finished = False
        start_time = time.time()

        while time.time() - start_time < 120:
            r, _, _ = select.select([master], [], [], 0.5)
            if master in r:
                try:
                    data = os.read(master, 1024)
                    if not data:
                        break
                    text = data.decode("utf-8", errors="replace")
                    sys.stdout.write(text)
                    sys.stdout.flush()
                    output.append(text)
                    combined = "".join(output)

                    # 1. Detect app started on phone
                    if not started and "Flutter run key commands." in combined:
                        started = True
                        print("\n[VERIFY] App started on device! Modifying lib/main.dart...\n")
                        with open(main_dart, "r") as f:
                            content = f.read()
                        new_content = content.replace(
                            "You have pushed the button this many times:",
                            "Cave hot reload verified: pushed button:"
                        )
                        with open(main_dart, "w") as f:
                            f.write(new_content)
                        time.sleep(1)
                        print("\n[VERIFY] Sending 'r' (hot reload)...\n")
                        os.write(master, b"r")

                    # 2. Detect reload complete
                    if started and not reloaded and ("Reloaded " in combined or "reloaded " in combined):
                        reloaded = True
                        print("\n[VERIFY] Hot reload confirmed! Sending 'q' (quit)...\n")
                        time.sleep(2)
                        os.write(master, b"q")
                        finished = True

                except OSError:
                    break

        os.close(master)
        _, status = os.waitpid(pid, 0)
        print(f"\n[VERIFY] Process exited with status {status}")
        print(f"[VERIFY] App started: {started}, Hot reloaded: {reloaded}")

if __name__ == "__main__":
    main()
