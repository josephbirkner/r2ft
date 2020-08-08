use crate::common::{Serializable, Cursor, SerializationResult};
use crate::transport::frame::*;
use itertools::Itertools;
use std::fs::File;
use std::io::{Seek, SeekFrom};

#[test]
fn test_serialize_message_frame()
{
    // Structure to serialize
    let message_frame = MessageFrame {
        version: 1,
        sid: 123,
        tlvs: vec![
            TransportTlv::ObjectHeader(ObjectHeader {
                object_id: 1,
                n_chunks: 2,
                ack_req: true,
                object_type: 42,
                fields: vec![
                    ObjectFieldDescription{
                        field_type: 66,
                        length: 6
                    }
                ]
            }),
            TransportTlv::ObjectHeader(ObjectHeader {
                object_id: 1,
                n_chunks: 2,
                ack_req: true,
                object_type: 42,
                fields: vec![
                    ObjectFieldDescription{
                        field_type: 66,
                        length: 6
                    }
                ]
            }),
        ]
    };

    // Serialize it!
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(buffer);
    message_frame.serialize(&mut cursor);
    println!("Encoded message frame: {:02x}", cursor.get_ref().iter().format(" "));

    // Deserialize it!
    cursor.seek(SeekFrom::Start(0));
    let mut parsed_message_frame = MessageFrame::default();
    match parsed_message_frame.deserialize(&mut cursor) {
        SerializationResult::Err(x) => println!("Error: {}", &x.to_string()),
        _ => {},
    }

    // Compare ...
    assert_eq!(message_frame.version, parsed_message_frame.version);
    assert_eq!(message_frame.sid, parsed_message_frame.sid);
    assert_eq!(message_frame.tlvs.len(), parsed_message_frame.tlvs.len());
    for tlv_id in 0..message_frame.tlvs.len() {
        let ltlv = &message_frame.tlvs[tlv_id];
        let rtlv = &parsed_message_frame.tlvs[tlv_id];
        match (ltlv, rtlv) {
            (TransportTlv::ObjectHeader(l), TransportTlv::ObjectHeader(r)) => {
                assert_eq!(l.object_id, r.object_id);
                assert_eq!(l.n_chunks, r.n_chunks);
                assert_eq!(l.ack_req, r.ack_req);
                assert_eq!(l.object_type, r.object_type);
                assert_eq!(l.fields.len(), r.fields.len());
                for field_id in 0..l.fields.len() {
                    assert_eq!(l.fields[field_id].field_type, r.fields[field_id].field_type);
                    assert_eq!(l.fields[field_id].length, r.fields[field_id].length);
                }
            }
        }
    }
}
