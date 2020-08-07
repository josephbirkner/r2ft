use crate::common::{Serializable, Cursor};
use crate::transport::frame::*;
use itertools::Itertools;
use std::fs::File;
use std::io::{Seek, SeekFrom};

#[test]
fn test_serialize_object_header()
{
    // Structure to serialize
    let object_header = ObjectHeader{
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
    };

    // Serialize it!
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(buffer);
    object_header.serialize(&mut cursor);
    println!("Encoded object header: {:02x}", cursor.get_ref().iter().format(" "));

    // Deserialize it!
    cursor.seek(SeekFrom::Start(0));
    let mut parsed_object_header = ObjectHeader::default();
    parsed_object_header.deserialize(&mut cursor);

    // Compare ...
    assert_eq!(object_header.object_id, parsed_object_header.object_id);
    assert_eq!(object_header.n_chunks, parsed_object_header.n_chunks);
    assert_eq!(object_header.ack_req, parsed_object_header.ack_req);
    assert_eq!(object_header.object_type, parsed_object_header.object_type);
    assert_eq!(object_header.fields.len(), parsed_object_header.fields.len());
    for i in 0..object_header.fields.len() {
        assert_eq!(object_header.fields[i].field_type, parsed_object_header.fields[i].field_type);
        assert_eq!(object_header.fields[i].length, parsed_object_header.fields[i].length);
    }
}
