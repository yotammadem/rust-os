SHELL := /bin/zsh

TARGET := x86_64-unknown-uefi
KERNEL_TARGET := x86_64-unknown-none
PROFILE ?= debug
BUILD_DIR := .build
EFI_STAGING := $(BUILD_DIR)/efi
IMAGE := bin/hello-boot.img
LOADER_EFI := target/$(TARGET)/$(PROFILE)/loader.efi
KERNEL_IMAGE := target/$(KERNEL_TARGET)/$(PROFILE)/kernel
BOOTX64_EFI := $(EFI_STAGING)/EFI/BOOT/BOOTX64.EFI
KERNEL_APP_IMAGE := $(EFI_STAGING)/EFI/BOOT/KERNEL.BIN

.PHONY: build clean loader kernel image check-tools

build: $(IMAGE)

loader: $(LOADER_EFI)

kernel: $(KERNEL_IMAGE)

image: $(IMAGE)

$(LOADER_EFI): Cargo.toml loader/Cargo.toml loader/build.rs loader/src/main.rs loader/src/bootinfo.rs loader/src/elf.rs loader/src/kernel_image.rs loader/src/memory.rs loader/src/paging.rs rust-toolchain.toml .cargo/config.toml src/lib.rs src/boot/mod.rs src/boot/handoff.rs src/boot/multiboot.rs src/arch/mod.rs src/arch/x86_64/mod.rs src/arch/x86_64/serial.rs src/arch/x86_64/halt.rs linker/loader.ld asm/boot.s
	cargo build --manifest-path loader/Cargo.toml --target $(TARGET)

$(KERNEL_IMAGE): Cargo.toml kernel/Cargo.toml kernel/build.rs kernel/src/main.rs rust-toolchain.toml .cargo/config.toml src/lib.rs src/boot/mod.rs src/boot/handoff.rs src/arch/mod.rs src/arch/x86_64/mod.rs src/arch/x86_64/serial.rs src/arch/x86_64/halt.rs linker/kernel.ld asm/boot.s
	cargo build --manifest-path kernel/Cargo.toml --target $(KERNEL_TARGET)

$(IMAGE): $(LOADER_EFI) $(KERNEL_IMAGE)
	@mkdir -p bin $(EFI_STAGING)/EFI/BOOT
	cp $(LOADER_EFI) $(BOOTX64_EFI)
	cp $(KERNEL_IMAGE) $(KERNEL_APP_IMAGE)
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
		cp $(BOOTX64_EFI) $$MOUNT_POINT/EFI/BOOT/BOOTX64.EFI; \
		cp $(KERNEL_APP_IMAGE) $$MOUNT_POINT/EFI/BOOT/KERNEL.BIN; \
		hdiutil detach $$DEV >/dev/null

clean:
	rm -rf $(BUILD_DIR) bin/hello-boot.img
	cargo clean
