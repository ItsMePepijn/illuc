#!/usr/bin/env python3
import argparse
import fcntl
import os
import pty
import select
import socket
import struct
import sys
import termios
import threading


def set_winsize(fd, rows, cols):
    try:
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
    except Exception:
        pass


def run_control_server(master_fd, port, stop_event):
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(("127.0.0.1", port))
    server.listen(1)
    actual_port = server.getsockname()[1]
    sys.stderr.write(f"PORT:{actual_port}\n")
    sys.stderr.flush()
    server.settimeout(0.5)
    while not stop_event.is_set():
        try:
            conn, _ = server.accept()
        except socket.timeout:
            continue
        with conn:
            buffer = b""
            conn.settimeout(0.5)
            while not stop_event.is_set():
                try:
                    data = conn.recv(256)
                except socket.timeout:
                    continue
                if not data:
                    break
                buffer += data
                while b"\n" in buffer:
                    line, buffer = buffer.split(b"\n", 1)
                    parts = line.strip().split()
                    if len(parts) != 2:
                        continue
                    try:
                        rows = int(parts[0])
                        cols = int(parts[1])
                    except ValueError:
                        continue
                    set_winsize(master_fd, rows, cols)
    server.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--control-port", type=int, required=True)
    parser.add_argument("--rows", type=int, required=True)
    parser.add_argument("--cols", type=int, required=True)
    parser.add_argument("--cwd", type=str, required=True)
    parser.add_argument("--term", type=str, default="")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()

    command = args.command
    if not command:
        sys.stderr.write("Missing command\n")
        sys.stderr.flush()
        return 2

    try:
        os.chdir(args.cwd)
    except Exception:
        pass

    if args.term:
        os.environ["TERM"] = args.term

    master_fd, slave_fd = pty.openpty()
    set_winsize(slave_fd, args.rows, args.cols)

    pid = os.fork()
    if pid == 0:
        os.setsid()
        os.dup2(slave_fd, 0)
        os.dup2(slave_fd, 1)
        os.dup2(slave_fd, 2)
        os.close(master_fd)
        os.close(slave_fd)
        os.execvp(command[0], command)
        os._exit(1)

    os.close(slave_fd)
    stop_event = threading.Event()
    control_thread = threading.Thread(
        target=run_control_server,
        args=(master_fd, args.control_port, stop_event),
        daemon=True,
    )
    control_thread.start()

    try:
        while True:
            read_list = [master_fd, sys.stdin.fileno()]
            ready, _, _ = select.select(read_list, [], [])
            if master_fd in ready:
                data = os.read(master_fd, 8192)
                if not data:
                    break
                os.write(sys.stdout.fileno(), data)
            if sys.stdin.fileno() in ready:
                data = os.read(sys.stdin.fileno(), 8192)
                if not data:
                    break
                os.write(master_fd, data)
    finally:
        stop_event.set()

    _, status = os.waitpid(pid, 0)
    if os.WIFEXITED(status):
        return os.WEXITSTATUS(status)
    return 1


if __name__ == "__main__":
    sys.exit(main())
