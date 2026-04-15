#[cfg(all(target_arch = "aarch64", feature = "rtc"))]
use core::ptr::NonNull;

use ax_plat::init::InitIf;
#[cfg(all(target_arch = "aarch64", feature = "rtc"))]
use ax_plat::mem::phys_to_virt;
#[cfg(all(target_arch = "aarch64", feature = "rtc"))]
use fdt_parser::Fdt;

struct InitIfImpl;

#[cfg(all(target_arch = "aarch64", feature = "rtc"))]
const QEMU_VIRT_PL031_PADDR: usize = 0x0910_0000;

#[cfg(all(target_arch = "aarch64", feature = "rtc"))]
fn init_rtc_mmio(paddr: usize, size: usize) {
    let Ok(rtc_base) = axklib::mem::iomap(paddr.into(), size) else {
        return;
    };

    ax_plat_aarch64_peripherals::pl031::init_early(rtc_base);
}

#[cfg(all(target_arch = "aarch64", feature = "rtc"))]
fn init_rtc_from_dtb(dtb: usize) {
    if dtb == 0 {
        init_rtc_mmio(QEMU_VIRT_PL031_PADDR, 0x1000);
        return;
    }

    let Some(dtb_ptr) = NonNull::new(phys_to_virt(dtb.into()).as_mut_ptr()) else {
        init_rtc_mmio(QEMU_VIRT_PL031_PADDR, 0x1000);
        return;
    };
    let Ok(fdt) = Fdt::from_ptr(dtb_ptr) else {
        init_rtc_mmio(QEMU_VIRT_PL031_PADDR, 0x1000);
        return;
    };
    let Some(node) = fdt.find_compatible(&["arm,pl031", "arm,primecell"]).next() else {
        init_rtc_mmio(QEMU_VIRT_PL031_PADDR, 0x1000);
        return;
    };
    let Some(mut regs) = node.reg() else {
        init_rtc_mmio(QEMU_VIRT_PL031_PADDR, 0x1000);
        return;
    };
    let Some(reg) = regs.next() else {
        init_rtc_mmio(QEMU_VIRT_PL031_PADDR, 0x1000);
        return;
    };

    init_rtc_mmio(reg.address as usize, reg.size.unwrap_or(0x1000) as usize);
}

#[impl_plat_interface]
impl InitIf for InitIfImpl {
    /// Initializes the platform at the early stage for the primary core.
    ///
    /// This function should be called immediately after the kernel has booted,
    /// and performed earliest platform configuration and initialization (e.g.,
    /// early console, clocking).
    fn init_early(_cpu_id: usize, _dtb: usize) {
        ax_cpu::init::init_trap();
        #[cfg(all(target_arch = "aarch64", feature = "fp-simd"))]
        {
            ax_cpu::asm::enable_fp();
            debug!("axplat-dyn: fp/simd enabled");
        }
        somehal::timer::enable();
    }

    /// Initializes the platform at the early stage for secondary cores.
    #[cfg(feature = "smp")]
    fn init_early_secondary(_cpu_id: usize) {
        ax_cpu::init::init_trap();
        #[cfg(all(target_arch = "aarch64", feature = "fp-simd"))]
        {
            ax_cpu::asm::enable_fp();
            debug!("axplat-dyn: secondary fp/simd enabled");
        }
        somehal::timer::enable();
    }

    /// Initializes the platform at the later stage for the primary core.
    ///
    /// This function should be called after the kernel has done part of its
    /// initialization (e.g, logging, memory management), and finalized the rest of
    /// platform configuration and initialization.
    fn init_later(_cpu_id: usize, _dtb: usize) {
        somehal::post_paging();
        #[cfg(all(target_arch = "aarch64", feature = "rtc"))]
        init_rtc_from_dtb(_dtb);
        somehal::timer::irq_enable();
    }

    /// Initializes the platform at the later stage for secondary cores.
    #[cfg(feature = "smp")]
    fn init_later_secondary(_cpu_id: usize) {
        somehal::timer::irq_enable();
    }
}
