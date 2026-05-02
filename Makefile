SHELL := /bin/zsh

TARGET := x86_64-unknown-uefi
PROFILE ?= debug
KERNEL_NAME := hello-boot
BUILD_DIR := .build
EFI_STAGING := $(BUILD_DIR)/efi
KERNEL_EFI := target/$(TARGET)/$(PROFILE)/$(KERNEL_NAME).efi
GRUB_EFI := $(EFI_STAGING)/EFI/BOOT/BOOTX64.EFI
APP_EFI := $(EFI_STAGING)/EFI/BOOT/HELLO.EFI
GRUB_MKSTANDALONE := $(shell command -v grub-mkstandalone 2>/dev/null || command -v x86_64-elf-grub-mkstandalone 2>/dev/null)

.PHONY: build clean kernel image check-tools

build: $(APP_EFI)

kernel: $(KERNEL_EFI)

image: $(APP_EFI)

$(KERNEL_EFI): Cargo.toml rust-toolchain.toml .cargo/config.toml src/lib.rs src/main.rs src/boot/mod.rs src/boot/uefi.rs src/arch/mod.rs src/arch/x86_64/mod.rs src/arch/x86_64/framebuffer.rs src/arch/x86_64/halt.rs src/arch/x86_64/serial.rs src/kernel/mod.rs src/kernel/hello.rs src/memory/mod.rs src/memory/map.rs src/memory/bitmap.rs asm/boot.s
	cargo build --target $(TARGET)

$(GRUB_EFI): grub/grub.cfg $(KERNEL_EFI)
	@test -n "$(GRUB_MKSTANDALONE)" || { echo "missing grub-mkstandalone; install GRUB host tooling first"; exit 1; }
	@mkdir -p $(dir $(GRUB_EFI))
	$(GRUB_MKSTANDALONE) -O x86_64-efi -o $(GRUB_EFI) \
		"boot/grub/grub.cfg=grub/grub.cfg" \
		"EFI/BOOT/HELLO.EFI=$(KERNEL_EFI)"

$(APP_EFI): $(KERNEL_EFI) $(GRUB_EFI)
	@mkdir -p $(EFI_STAGING)/EFI/BOOT
	cp $(KERNEL_EFI) $(APP_EFI)

clean:
	rm -rf $(BUILD_DIR)
	cargo clean
