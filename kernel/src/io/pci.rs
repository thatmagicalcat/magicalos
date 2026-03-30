// TODO: check for MMIO support and use it if available

use super::port::Port;

const CONFIG_ADDRESS: usize = 0xCF8;
const CONFIG_DATA: usize = 0xCFC;

/*
 * Config address bits:
 * 31    = enable bit
 * 30-24 = reserved
 * 23-16 = bus number
 * 15-11 = device number
 * 10-8  = function number
 * 7-0   = register offset
 *         (bits 1:0 are always 0b00)
 * */

