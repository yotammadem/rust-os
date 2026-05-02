import os
import pty
import select
import signal
import shutil
import subprocess
import time
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
HELLO_WORLD = "hello world"
PAGING_ROOT = "paging root:"
BUILD_TIMEOUT_SECS = 60
BOOT_TIMEOUT_SECS = 12
TARGET = "x86_64-unknown-uefi"
KERNEL_EFI = REPO_ROOT / "target" / TARGET / "debug" / "hello-boot.efi"
EFI_STAGING = REPO_ROOT / ".build" / "efi" / "EFI" / "BOOT"
GRUB_EFI = EFI_STAGING / "BOOTX64.EFI"
APP_EFI = EFI_STAGING / "HELLO.EFI"
DEFAULT_OVMF_CODE = "/usr/local/share/qemu/edk2-x86_64-code.fd"


class BootSerialE2ETest(unittest.TestCase):
    def test_qemu_boot_eventually_prints_hello_world_to_serial(self) -> None:
        self._build_efi_tree()

        transcript = self._run_qemu_under_pty()
        self.assertIn(
            PAGING_ROOT,
            transcript,
            msg=(
                f"serial transcript did not contain `{PAGING_ROOT}` within "
                f"{BOOT_TIMEOUT_SECS}s:\n{transcript}"
            ),
        )
        self.assertIn(
            HELLO_WORLD,
            transcript,
            msg=(
                f"serial transcript did not contain `{HELLO_WORLD}` within "
                f"{BOOT_TIMEOUT_SECS}s:\n{transcript}"
            ),
        )

    def _build_efi_tree(self) -> None:
        subprocess.run(
            ["cargo", "build", "--target", TARGET],
            cwd=REPO_ROOT,
            check=True,
            timeout=BUILD_TIMEOUT_SECS,
        )

        EFI_STAGING.mkdir(parents=True, exist_ok=True)
        grub_tool = shutil.which("x86_64-elf-grub-mkstandalone") or shutil.which(
            "grub-mkstandalone"
        )
        self.assertIsNotNone(grub_tool, "missing grub-mkstandalone host tool")
        subprocess.run(
            [
                grub_tool,
                "-O",
                "x86_64-efi",
                "-o",
                str(GRUB_EFI),
                "boot/grub/grub.cfg=grub/grub.cfg",
                f"EFI/BOOT/HELLO.EFI={KERNEL_EFI}",
            ],
            cwd=REPO_ROOT,
            check=True,
            timeout=BUILD_TIMEOUT_SECS,
        )
        shutil.copy2(KERNEL_EFI, APP_EFI)

    def _run_qemu_under_pty(self) -> str:
        firmware_path = os.environ.get("OVMF_CODE", DEFAULT_OVMF_CODE)
        master_fd, slave_fd = pty.openpty()
        process = subprocess.Popen(
            [
                "qemu-system-x86_64",
                "-machine",
                "q35,accel=hvf",
                "-m",
                "256M",
                "-serial",
                "stdio",
                "-drive",
                f"if=pflash,format=raw,readonly=on,file={firmware_path}",
                "-drive",
                f"format=raw,file=fat:rw:{REPO_ROOT / '.build' / 'efi'}",
            ],
            cwd=REPO_ROOT,
            stdin=slave_fd,
            stdout=slave_fd,
            stderr=slave_fd,
            start_new_session=True,
            text=False,
        )
        os.close(slave_fd)

        deadline = time.monotonic() + BOOT_TIMEOUT_SECS
        chunks: list[bytes] = []

        try:
            while time.monotonic() < deadline:
                remaining = max(0.0, deadline - time.monotonic())
                ready, _, _ = select.select([master_fd], [], [], min(0.2, remaining))
                if ready:
                    try:
                        chunk = os.read(master_fd, 4096)
                    except OSError:
                        break
                    if not chunk:
                        break
                    chunks.append(chunk)
                    transcript = b"".join(chunks).decode("utf-8", errors="replace")
                    if HELLO_WORLD in transcript:
                        return transcript

                exit_code = process.poll()
                if exit_code is not None:
                    break

            return b"".join(chunks).decode("utf-8", errors="replace")
        finally:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait(timeout=2)
            os.close(master_fd)


if __name__ == "__main__":
    unittest.main()
