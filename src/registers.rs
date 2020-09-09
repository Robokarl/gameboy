use std::fmt;

#[derive(Default, Copy, Clone, Debug)]
pub struct Flags {
    pub z: bool,
    pub n: bool,
    pub h: bool,
    pub c: bool,
}

impl Flags {
    pub fn as_u8(&self) -> u8 {
        let mut val = 0;
        if self.z { val |= 0x80 };
        if self.n { val |= 0x40 };
        if self.h { val |= 0x20 };
        if self.c { val |= 0x10 };
        val
    }

    pub fn set_from_u8(&mut self, value: u8) {
        self.z = value & 0x80 == 0x80;
        self.n = value & 0x40 == 0x40;
        self.h = value & 0x20 == 0x20;
        self.c = value & 0x10 == 0x10;
    }
}

#[derive(Default, Debug)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: Flags,
    pub h: u8,
    pub l: u8,
}

impl fmt::LowerHex for Registers {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("Registers {{ a: {:02x}, b: {:02x}, c: {:02x}, d: {:02x}, e: {:02x}, f: {:02x}, h: {:02x}, l: {:02x} }}", self.a, self.b, self.c, self.d, self.e, self.f.as_u8(), self.h, self.l))?;
        Ok(())
    }
}

impl Registers {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | (self.f.as_u8() as u16)
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = ((value & 0xFF00) >> 8) as u8;
        self.f.set_from_u8(value as u8);
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = value as u8;
    }

    pub fn get_bc(&self) -> u16 {
        (self.b as u16) << 8 | (self.c as u16)
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = ((value & 0xFF00) >> 8) as u8;
        self.e = value as u8;
    }

    pub fn get_de(&self) -> u16 {
        (self.d as u16) << 8 | (self.e as u16)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = ((value & 0xFF00) >> 8) as u8;
        self.l = value as u8;
    }

    pub fn get_hl(&self) -> u16 {
        (self.h as u16) << 8 | (self.l as u16)
    }

    pub fn get_reg8_by_id(&self, id: u8) -> u8 {
        match id {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            7 => self.a,
            _ => panic!("Invalid register ID: {}", id),
        }
    }

    pub fn set_reg8_by_id(&mut self, id: u8, value: u8) {
        match id {
            0 => self.b = value,
            1 => self.c = value,
            2 => self.d = value,
            3 => self.e = value,
            4 => self.h = value,
            5 => self.l = value,
            7 => self.a = value,
            _ => panic!("Invalid register ID: {}", id),
        }
    }

    pub fn set_reg16_by_id(&mut self, id: u8, value: u16) {
        match id {
            0 => self.set_bc(value),
            1 => self.set_de(value),
            2 => self.set_hl(value),
            3 => self.set_af(value),
            _ => panic!("Invalid register ID: {}", id),
        }
    }

    pub fn get_reg16_by_id(&self, id: u8) -> u16 {
        match id {
            0 => self.get_bc(),
            1 => self.get_de(),
            2 => self.get_hl(),
            3 => self.get_af(),
            _ => panic!("Invalid register ID: {}", id),
        }
    }

    pub fn get_condition_by_id(&self, id: u8) -> bool {
        match id {
            0 => !self.f.z,
            1 => self.f.z,
            2 => !self.f.c,
            3 => self.f.c,
            _ => panic!("Invalid condition ID: {}", id),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hl() {
        let mut regs = Registers::new();
        regs.set_hl(0x55aa);
        assert_eq!(regs.h, 0x55);
        assert_eq!(regs.l, 0xaa);
        assert_eq!(regs.get_hl(), 0x55aa);
    }

    #[test]
    fn test_af() {
        let mut regs = Registers::new();
        regs.a = 0x12;
        regs.f.set_from_u8(0xa0);
        assert_eq!(regs.get_af(), 0x12a0);
    }

    #[test]
    fn test_bc() {
        let mut regs = Registers::new();
        regs.set_bc(0x5678);
        assert_eq!(regs.b, 0x56);
        assert_eq!(regs.c, 0x78);
        assert_eq!(regs.get_bc(), 0x5678);
    }

    #[test]
    fn test_de() {
        let mut regs = Registers::new();
        regs.set_de(0xabcd);
        assert_eq!(regs.d, 0xab);
        assert_eq!(regs.e, 0xcd);
        assert_eq!(regs.get_de(), 0xabcd);
    }
}
