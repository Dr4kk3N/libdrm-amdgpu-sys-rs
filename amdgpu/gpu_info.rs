use crate::bindings::{self, amdgpu_gpu_info, drm_amdgpu_info_device};
use crate::AMDGPU::{self, ASIC_NAME, GfxTargetVersion};

/// Information that [amdgpu_gpu_info] and [drm_amdgpu_info_device] have in common
pub trait GPU_INFO {
    fn family_id(&self) -> u32;
    fn chip_external_rev(&self) -> u32;
    fn device_id(&self) -> u32;
    fn pci_rev_id(&self) -> u32;
    fn vram_type(&self) -> u32;
    /// Note: AMDGPU driver reports VRAM width per memory channel for LPDDR5 as 64-bits.
    /// <https://gitlab.freedesktop.org/drm/amd/-/issues/2468>
    fn vram_bit_width(&self) -> u32;
    /// KHz
    fn max_memory_clock(&self) -> u64;
    /// KHz
    fn max_engine_clock(&self) -> u64;
    fn ids_flags(&self) -> u64;
    fn rb_pipes(&self) -> u32;
    fn cu_active_number(&self) -> u32;
    fn max_se(&self) -> u32;
    fn max_sa_per_se(&self) -> u32;

    fn get_family_name(&self) -> AMDGPU::FAMILY_NAME {
        AMDGPU::FAMILY_NAME::from(self.family_id())
    }

    fn get_asic_name(&self) -> AMDGPU::ASIC_NAME {
        self.get_family_name().asic_name(self.chip_external_rev())
    }

    fn get_chip_class(&self) -> AMDGPU::CHIP_CLASS {
        self.get_asic_name().chip_class()
    }

    fn get_vram_type(&self) -> AMDGPU::VRAM_TYPE {
        AMDGPU::VRAM_TYPE::from(self.vram_type())
    }

    fn is_apu(&self) -> bool {
        use crate::bindings::AMDGPU_IDS_FLAGS_FUSION;

        (self.ids_flags() & AMDGPU_IDS_FLAGS_FUSION as u64) != 0
    }

    fn peak_memory_bw(&self) -> u64 {
        let vram_type = self.get_vram_type();

        vram_type.peak_bw(self.max_memory_clock(), self.vram_bit_width())
    }

    fn peak_memory_bw_gb(&self) -> u64 {
        self.peak_memory_bw() / 1000
    }

    fn calc_rop_count(&self) -> u32 {
        let rop_per_rb = if self.get_asic_name().rbplus_allowed() { 8 } else { 4 };

        self.rb_pipes() * rop_per_rb
    }

    /// \[CU\] * \[Lane\] * 2 \[ops\] * \[GHz\]
    fn peak_gflops(&self) -> u32 {
        let cu = self.cu_active_number();
        let lane = if self.get_chip_class() >= AMDGPU::CHIP_CLASS::GFX11 { 128 } else { 64 };
        let mhz = (self.max_engine_clock() / 1000) as u32;
        (cu * lane * 2 * mhz) / 1000
    }

    /// Find device marketing name from `amdgpu.ids`  
    /// Link: <https://gitlab.freedesktop.org/mesa/drm/-/blob/main/data/amdgpu.ids>
    fn find_device_name(&self) -> Option<String> {
        let did = self.device_id();
        let rid = self.pci_rev_id();

        find_device_name(did, rid)
    }

    /// Returns the default marketing name ("AMD Radeon Graphics") 
    /// when the device name is not available.
    fn find_device_name_or_default(&self) -> String {
        self.find_device_name().unwrap_or(AMDGPU::DEFAULT_DEVICE_NAME.to_string())
    }

    fn get_max_good_cu_per_sa(&self) -> u32 {
        let cu_group = self.get_chip_class().cu_group() as u32;
        let max_sa = self.max_se() * self.max_sa_per_se();

        self.cu_active_number().div_ceil(max_sa * cu_group) * cu_group
    }

    fn get_min_good_cu_per_sa(&self) -> u32 {
        let cu_group = self.get_chip_class().cu_group() as u32;
        let max_sa = self.max_se() * self.max_sa_per_se();

        self.cu_active_number() / (max_sa * cu_group) * cu_group
    }

    fn get_l1_cache_size(&self) -> u32 {
        self.get_asic_name().l1_cache_size()
    }

    fn get_gl1_cache_size(&self) -> u32 {
        self.get_asic_name().gl1_cache_size()
    }
}

impl GPU_INFO for amdgpu_gpu_info {
    fn family_id(&self) -> u32 {
        self.family_id
    }
    fn chip_external_rev(&self) -> u32 {
        self.chip_external_rev
    }
    fn device_id(&self) -> u32 {
        self.asic_id
    }
    fn pci_rev_id(&self) -> u32 {
        self.pci_rev_id
    }
    fn vram_type(&self) -> u32 {
        self.vram_type
    }
    fn vram_bit_width(&self) -> u32 {
        self.vram_bit_width
    }
    fn max_memory_clock(&self) -> u64 {
        self.max_memory_clk
    }
    fn max_engine_clock(&self) -> u64 {
        self.max_engine_clk
    }
    fn ids_flags(&self) -> u64 {
        self.ids_flags
    }
    fn rb_pipes(&self) -> u32 {
        self.rb_pipes
    }
    fn cu_active_number(&self) -> u32 {
        self.cu_active_number
    }
    fn max_se(&self) -> u32 {
        self.num_shader_engines
    }
    fn max_sa_per_se(&self) -> u32 {
        self.num_shader_arrays_per_engine
    }
}

impl GPU_INFO for drm_amdgpu_info_device {
    fn family_id(&self) -> u32 {
        self.family
    }
    fn chip_external_rev(&self) -> u32 {
        self.external_rev
    }
    fn device_id(&self) -> u32 {
        self.device_id
    }
    fn pci_rev_id(&self) -> u32 {
        self.pci_rev
    }
    fn vram_type(&self) -> u32 {
        self.vram_type
    }
    fn vram_bit_width(&self) -> u32 {
        self.vram_bit_width
    }
    fn max_memory_clock(&self) -> u64 {
        self.max_memory_clock
    }
    fn max_engine_clock(&self) -> u64 {
        self.max_engine_clock
    }
    fn ids_flags(&self) -> u64 {
        self.ids_flags
    }
    fn rb_pipes(&self) -> u32 {
        self.num_rb_pipes
    }
    fn cu_active_number(&self) -> u32 {
        self.cu_active_number
    }
    fn max_se(&self) -> u32 {
        self.num_shader_engines
    }
    fn max_sa_per_se(&self) -> u32 {
        self.num_shader_arrays_per_engine
    }
}

impl drm_amdgpu_info_device {
    pub fn get_l1_cache_size(&self) -> u32 {
        if self.is_gfx11_or_later() && self.tcp_cache_size != 0 {
            self.tcp_cache_size << 10
        } else {
            self.get_asic_name().l1_cache_size()
        }
    }

    pub fn get_gl1_cache_size(&self) -> u32 {
        if self.is_gfx11_or_later() && self.gl1c_cache_size != 0 {
            self.gl1c_cache_size << 10
        } else {
            self.get_asic_name().gl1_cache_size()
        }
    }

    pub fn get_max_tcc_blocks(&self) -> u32 {
        self.num_tcc_blocks
    }

    /// num_tcc_blocks - self.tcc_disabled_mask.count_ones() = memory channels
    pub fn get_actual_num_tcc_blocks(&self) -> u32 {
        self.num_tcc_blocks - self.tcc_disabled_mask.count_ones()
    }

    pub fn calc_l2_cache_size(&self) -> u32 {
        if self.is_gfx11_or_later() && self.gl2c_cache_size != 0 {
            self.gl2c_cache_size << 10
        } else {
            self.get_actual_num_tcc_blocks() * self.get_asic_name().l2_cache_size_per_block()
        }
    }

    pub fn calc_l3_cache_size_mb(&self) -> u32 {
        if self.is_gfx11_or_later() && self.mall_size != 0 {
            (self.mall_size >> 20) as u32
        } else {
            self.get_actual_num_tcc_blocks() * self.get_asic_name().l3_cache_size_mb_per_channel()
        }
    }

    fn is_gfx11_or_later(&self) -> bool {
        self.get_asic_name() >= ASIC_NAME::CHIP_GFX1100
    }

    /// ref: drivers/gpu/drm/amd/amd/amdkfd/kfd_device.c
    pub fn get_gfx_target_version(&self) -> Option<GfxTargetVersion> {
        use AMDGPU::ASIC_NAME;

        let asic = self.get_asic_name();

        let gfx_ver = match asic {
            ASIC_NAME::CHIP_KAVERI => (7, 0, 0),
            ASIC_NAME::CHIP_CARRIZO => (8, 0, 1),
            ASIC_NAME::CHIP_HAWAII => (7, 0, 1),
            ASIC_NAME::CHIP_TONGA => (8, 0, 2),
            ASIC_NAME::CHIP_FIJI |
            ASIC_NAME::CHIP_POLARIS10 |
            ASIC_NAME::CHIP_POLARIS11 |
            ASIC_NAME::CHIP_POLARIS12 |
            ASIC_NAME::CHIP_VEGAM => (8, 0, 3),
            ASIC_NAME::CHIP_VEGA10 => (9, 0, 0),
            ASIC_NAME::CHIP_RAVEN |
            ASIC_NAME::CHIP_RAVEN2 => (9, 0, 2),
            ASIC_NAME::CHIP_RENOIR => (9, 0, 12),
            ASIC_NAME::CHIP_VEGA20 => (9, 0, 6),
            ASIC_NAME::CHIP_ARCTURUS => (9, 0, 8),
            ASIC_NAME::CHIP_ALDEBARAN => (9, 0, 10),
            ASIC_NAME::CHIP_GFX940 => if self.chip_rev > 1 {
                (9, 4, 2)
            } else if self.is_apu() {
                (9, 4, 0)
            } else {
                (9, 4, 1)
            },
            ASIC_NAME::CHIP_NAVI10 => (10, 1, 0),
            ASIC_NAME::CHIP_NAVI12 => (10, 1, 1),
            ASIC_NAME::CHIP_NAVI14 => (10, 1, 2),
            ASIC_NAME::CHIP_GFX1013 => (10, 1, 3),
            ASIC_NAME::CHIP_NAVI21 => (10, 3, 0),
            ASIC_NAME::CHIP_NAVI22 => (10, 3, 1),
            ASIC_NAME::CHIP_VANGOGH => (10, 3, 3),
            ASIC_NAME::CHIP_NAVI23 => (10, 3, 2),
            ASIC_NAME::CHIP_NAVI24 => (10, 3, 4),
            ASIC_NAME::CHIP_REMBRANDT => (10, 3, 5),
            ASIC_NAME::CHIP_GFX1036 => (10, 3, 6),
            ASIC_NAME::CHIP_GFX1100 => (11, 0, 0),
            ASIC_NAME::CHIP_GFX1101 => (11, 0, 1),
            ASIC_NAME::CHIP_GFX1102 => (11, 0, 2),
            ASIC_NAME::CHIP_GFX1103_R1 |
            ASIC_NAME::CHIP_GFX1103_R2 => (11, 0, 3),
            ASIC_NAME::CHIP_GFX1150 => (11, 5, 0),
            ASIC_NAME::CHIP_GFX1151 => (11, 5, 1),
            ASIC_NAME::CHIP_GFX1152 => (11, 5, 2),
            ASIC_NAME::CHIP_GFX1153 => (11, 5, 3),
            ASIC_NAME::CHIP_GFX1200 => (12, 0, 0),
            ASIC_NAME::CHIP_GFX1201 => (12, 0, 1),
            _ => return None,
        };

        Some(GfxTargetVersion::from(gfx_ver))
    }
}

/// Find device marketing name from `amdgpu.ids`  
/// Link: <https://gitlab.freedesktop.org/mesa/drm/-/blob/main/data/amdgpu.ids>
pub fn find_device_name(device_id: u32, revision_id: u32) -> Option<String> {
    use bindings::{AMDGPU_IDS, AMDGPU_IDS_2};

    let (_, _, name) = AMDGPU_IDS
        .iter()
        .find(|(did, rid, _)| (did, rid) == (&device_id, &revision_id))
        .or_else(|| {
            AMDGPU_IDS_2
                .iter()
                .find(|(did, rid, _)| (did, rid) == (&device_id, &revision_id))
        })?;

    Some(name.to_string())
}

/// Returns the default marketing name ("AMD Radeon Graphics") 
/// when the device name is not available.
pub fn find_device_name_or_default(device_id: u32, revision_id: u32) -> String {
    find_device_name(device_id, revision_id).unwrap_or(AMDGPU::DEFAULT_DEVICE_NAME.to_string())
}
