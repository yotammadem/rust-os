SHELL := /bin/zsh

TARGET := x86_64-unknown-uefi
PROFILE ?= debug
KERNEL_NAME := hello-boot
BUILD_DIR := .build
EFI_STAGING := $(BUILD_DIR)/efi
IMAGE := bin/hello-boot.img
KERNEL_EFI := target/$(TARGET)/$(PROFILE)/$(KERNEL_NAME).efi
GRUB_EFI := $(EFI_STAGING)/EFI/BOOT/BOOTX64.EFI
APP_EFI := $(EFI_STAGING)/EFI/BOOT/HELLO.EFI
GRUB_MKSTANDALONE := $(shell command -v grub-mkstandalone 2>/dev/null || command -v x86_64-elf-grub-mkstandalone 2>/dev/null)

.PHONY: build clean kernel image check-tools

build: $(IMAGE)

kernel: $(KERNEL_EFI)

image: $(IMAGE)

$(KERNEL_EFI): Cargo.toml rust-toolchain.toml .cargo/config.toml src/lib.rs src/main.rs src/boot/mod.rs src/boot/multiboot.rs src/arch/x86_64/mod.rs src/arch/x86_64/serial.rs src/arch/x86_64/halt.rs src/kernel/mod.rs src/kernel/hello.rs asm/boot.s
	cargo build --target $(TARGET)

$(GRUB_EFI): grub/grub.cfg $(KERNEL_EFI)
	@test -n "$(GRUB_MKSTANDALONE)" || { echo "missing grub-mkstandalone; install GRUB host tooling first"; exit 1; }
	@mkdir -p $(dir $(GRUB_EFI))
	$(GRUB_MKSTANDALONE) -O x86_64-efi -o $(GRUB_EFI) \
		"boot/grub/grub.cfg=grub/grub.cfg" \
		"EFI/BOOT/HELLO.EFI=$(KERNEL_EFI)"

$(IMAGE): $(KERNEL_EFI) $(GRUB_EFI)
	@mkdir -p bin $(EFI_STAGING)/EFI/BOOT
	cp $(KERNEL_EFI) $(APP_EFI)
	rm -f $(IMAGE)
	dd if=/dev/zero of=$(IMAGE) bs=1m count=64
	@DEV=$$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount $(IMAGE) | awk '/^\/dev\// { print $$1; exit }'); \
		if [ -z "$$DEV" ]; then echo "failed to attach raw image"; exit 1; fi; \
		diskutil partitionDisk $$DEV GPT FAT32 EFI R >/dev/null; \
		PART=$${DEV}s1; \
		diskutil mount $$PART >/dev/null; \
		MOUNT_POINT=$$(diskutil info $$PART | awk -F': *' '/Mount Point/ { print $$2 }'); \
		if [ -z "$$MOUNT_POINT" ]; then echo "failed to mount EFI partition"; hdiutil detach $$DEV >/dev/null; exit 1; fi; \
		mkdir -p $$MOUNT_POINT/EFI/BOOT; \
		cp $(GRUB_EFI) $$MOUNT_POINT/EFI/BOOT/BOOTX64.EFI; \
		cp $(APP_EFI) $$MOUNT_POINT/EFI/BOOT/HELLO.EFI; \
		hdiutil detach $$DEV >/dev/null

clean:
	rm -rf $(BUILD_DIR) bin/hello-boot.img
	cargo clean
