OVMF := assets/OVMF.fd
ESP := esp
BUILD_ARGS :=
QEMU_ARGS := -m 64M
QEMU_OUTPUT := -nographic
MODE ?= release
CUR_PATH := $(shell pwd)
DBG_INFO := false

ifeq (${MODE}, release)
	BUILD_ARGS := --release
endif

.PHONY: build run debug clean launch intdbg \
	target/x86_64-unknown-uefi/$(MODE)/ysos_boot.efi \
	target/x86_64-unknown-none/$(MODE)/ysos_kernel

run: build launch

launch:
	@qemu-system-x86_64 \
		-bios ${OVMF} \
		-net none \
		$(QEMU_ARGS) \
		$(QEMU_OUTPUT) \
		-drive format=raw,file=fat:rw:${ESP}

intdbg:
	@qemu-system-x86_64 \
		-bios ${OVMF} \
		-net none \
		$(QEMU_ARGS) \
		$(QEMU_OUTPUT) \
		-drive format=raw,file=fat:rw:${ESP} \
		-no-reboot -d int,cpu_reset

debug:
	@qemu-system-x86_64 \
		-bios ${OVMF} \
		-net none \
		$(QEMU_ARGS) \
		$(QEMU_OUTPUT) \
		-drive format=raw,file=fat:rw:${ESP} \
		-s -S

clean:
	@cargo clean

build: $(ESP)

$(ESP): $(ESP)/EFI/BOOT/BOOTX64.EFI $(ESP)/KERNEL.ELF $(ESP)/EFI/BOOT/boot.conf

$(ESP)/EFI/BOOT/BOOTX64.EFI: target/x86_64-unknown-uefi/$(MODE)/ysos_boot.efi
	@mkdir -p $(@D)
	cp $< $@
$(ESP)/EFI/BOOT/boot.conf: pkg/kernel/config/boot.conf
	@mkdir -p $(@D)
	cp $< $@
$(ESP)/KERNEL.ELF: target/x86_64-unknown-none/$(MODE)/ysos_kernel
	@mkdir -p $(@D)
	cp $< $@

target/x86_64-unknown-uefi/$(MODE)/ysos_boot.efi: pkg/boot
	cd pkg/boot && cargo build $(BUILD_ARGS)
target/x86_64-unknown-none/$(MODE)/ysos_kernel: pkg/kernel
	cd pkg/kernel && cargo build $(if $(ifeq($(DBG_INFO), true)), --profile=release-with-debug, $(BUILD_ARGS))
