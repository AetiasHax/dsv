mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};

    use anyhow::Result;
    use dzv_core::gdb::client::GdbClient;

    #[test]
    fn test_read_memory() -> Result<()> {
        let mut client = GdbClient::new();
        client.connect(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 3333))?;
        println!("Connected to GDB server");
        let x = client.read_u32(0x027e0f94)?;
        let y = client.read_u32(0x027e0f98)?;
        let z = client.read_u32(0x027e0f9c)?;
        println!("Read memory: {x:x?}, {y:x?}, {z:x?}");
        client.disconnect()?;
        Ok(())
    }
}
