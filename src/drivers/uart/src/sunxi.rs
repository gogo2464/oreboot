use clock::ClockNode;
use consts::DeviceCtl;
use core::ops;
use model::*;

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::register_bitfields;
use tock_registers::registers::{ReadOnly, ReadWrite};

const RETRY_COUNT: u32 = 100_000;

/*
 * see also https://github.com/smaeul/sun20i_d1_spl for reference
 * files: nboot/main/boot0_head.c and drivers/serial.c
 */

// D1 user manual
// https://dl.linux-sunxi.org/D1/D1_User_Manual_V0.1_Draft_Version.pdf

// pp 910
pub const UART0: u32 = 0x02500000;
pub const UART0_RB_TH_DLL: u32 = UART0 + 0x0;
pub const UART0_DLH_IE: u32 = UART0 + 0x4;
pub const UART0_II_FC: u32 = UART0 + 0x8;
pub const UART0_LC: u32 = UART0 + 0xc;
pub const UART0_LS: u32 = UART0 + 0x14;

// p897
pub const UART1: u32 = 0x02500400;
pub const UART2: u32 = 0x02500800;
pub const UART3: u32 = 0x02500C00;
pub const UART4: u32 = 0x02501000;
pub const UART5: u32 = 0x02501400;

#[repr(C)]
pub struct RegisterBlock {
    /* UART0 Receiver Buffer / Transmit Holding / Divisor Latch Low Register */
    u0rbthdll: ReadWrite<u32, UART0_RB_TH_DLL::Register>,
    /* UART0 Divisor Latch Low / Interrupt Enable Register */
    u0dlhie: ReadWrite<u32, UART0_DLH_IE::Register>,
    /* UART0 Interrupt Identity / FIFO Control Register */
    u0iifc: ReadWrite<u32, UART0_II_FC::Register>,
    /* UART0 Line Control Register */
    u0lc: ReadWrite<u32, UART0_LC::Register>,
    pad1: u32,
    /* UART0 Line Status Register */
    u0ls: ReadWrite<u32, UART0_LS::Register>,
}

pub struct Sunxi {
    base: usize,
    baudrate: u32,
}

impl ops::Deref for Sunxi {
    type Target = RegisterBlock;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr() }
    }
}

register_bitfields! [
    u32,
    UART0_RB_TH_DLL [
        VAL OFFSET(0) NUMBITS(8) []
    ],
    UART0_DLH_IE [
        DLH OFFSET(0) NUMBITS(8) [],
        /* IE */
        ERBFI OFFSET(0) NUMBITS(1) [],
        ETBEI OFFSET(1) NUMBITS(1) [],
        ELSI OFFSET(2) NUMBITS(1) [],
        EDSSI OFFSET(3) NUMBITS(1) [],
        RS485_INT_EN OFFSET(4) NUMBITS(1) [],
        // 6:5 are reserved
        PTIME OFFSET(7) NUMBITS(1) []
        // last 24 bits are reserved
    ],
    UART0_II_FC [
        IID OFFSET(0) NUMBITS(3) []
    ],
    UART0_LC [
        DLS OFFSET(0) NUMBITS(2) [],
        STOP OFFSET(2) NUMBITS(1) [],
        PEN OFFSET(3) NUMBITS(1) [],
        EPS OFFSET(4) NUMBITS(2) [],
        BC OFFSET(6) NUMBITS(1) [],
        DLAB OFFSET(7) NUMBITS(1) []
    ],
    UART0_LS [
        DR OFFSET(0) NUMBITS(1) [], // data ready
        TEMT OFFSET(6) NUMBITS(1) [] // transmitter empty
    ]
];

impl Sunxi {
    pub fn new(base: usize, baudrate: u32) -> Sunxi {
        Sunxi { base, baudrate }
    }

    /// Returns a pointer to the register block
    fn ptr(&self) -> *const RegisterBlock {
        self.base as *const _
    }
}

impl Driver for Sunxi {
    fn init(&mut self) -> Result<()> {
        // disable interrupts
        self.u0dlhie.modify(UART0_DLH_IE::ETBEI.val(0));

        // TODO: full init needs this; put it in the mainboard main.rs or in the
        // CCU init, assuming that we always need it anyway? How about panic?
        /*
        // reset
        self.bgr.modify(CCU_UART_BGR::UART0_RST.val(0));
        for i in 1..100 {}
        self.bgr.modify(CCU_UART_BGR::UART0_RST.val(1));

        // gate
        self.bgr.modify(CCU_UART_BGR::UART0_GATING.val(0));
        for i in 1..100 {}
        self.bgr.modify(CCU_UART_BGR::UART0_GATING.val(1));
        */

        Ok(())
    }

    fn pread(&self, data: &mut [u8], _offset: usize) -> Result<usize> {
        // TODO: this is just copied from another board
        /*
        'outer: for (read_count, c) in data.iter_mut().enumerate() {
            for _ in 0..RETRY_COUNT {
                // Create a copy of the rxdata register so that we don't
                // lose the Data field when we read the Empty field.
                let rd_copy = self.rd.extract();
                if !rd_copy.is_set(RD::Empty) {
                    *c = rd_copy.read(RD::Data) as u8;
                    continue 'outer;
                }
            }
            return Ok(read_count);
        }
        */
        Ok(data.len())
    }

    fn pwrite(&mut self, data: &[u8], _offset: usize) -> Result<usize> {
        'outer: for (sent_count, &c) in data.iter().enumerate() {
            for _ in 0..RETRY_COUNT {
                if self.u0ls.is_set(UART0_LS::TEMT) {
                    self.u0rbthdll.set(c.into());
                    continue 'outer;
                }
            }
            return Ok(sent_count);
        }
        Ok(data.len())
    }

    fn ctl(&mut self, __d: DeviceCtl) -> Result<usize> {
        NOT_IMPLEMENTED
    }

    fn stat(&self, _data: &mut [u8]) -> Result<usize> {
        NOT_IMPLEMENTED
    }

    fn shutdown(&mut self) {}
}
