// `register_type = "coil"` is not supported yet (the Vec<u16> register model cannot
// represent bit-addressed coils) and must be rejected at compile time rather than
// silently producing an incorrect mapping.
use modbus_mapper::ModbusMapper;

#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "coil")]
struct Coils {
    #[modbus(address = 0)]
    running: bool,
}

fn main() {}
