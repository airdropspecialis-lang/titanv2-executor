use crate::packet::{Meta as ProtoMeta, Packet as ProtoPacket, PacketFlags as ProtoPacketFlags};

pub fn proto_packet_from_versioned_tx(tx: &solana_sdk::transaction::VersionedTransaction) -> ProtoPacket {
    let data = bincode::serialize(&tx).unwrap();
    ProtoPacket {
        data,
        meta: Some(ProtoMeta {
            port: 0,
            addr: "0.0.0.0".to_string(),
            size: 0,
            flags: Some(ProtoPacketFlags {
                discard: false,
                forwarded: false,
                repair: false,
                simple_vote_tx: false,
                tracer_packet: false,
            }),
            sender_stake: 0,
        }),
    }
}