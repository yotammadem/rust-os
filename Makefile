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
KERNEL_SOURCES := $(shell find src asm .cargo -type f)

.PHONY: build clean kernel image check-tools

build: $(APP_EFI)

kernel: $(KERNEL_EFI)

image: $(APP_EFI)

$(KERNEL_EFI): Cargo.toml rust-toolchain.toml $(KERNEL_SOURCES)
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
