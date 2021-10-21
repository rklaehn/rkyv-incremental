use std::sync::Arc;

use rkyv::{
    archived_root, check_archived_root,
    de::deserializers::SharedDeserializeMap,
    ser::{
        serializers::{AlignedSerializer, AllocSerializer, CompositeSerializer},
        Serializer,
    },
    Deserialize,
};

fn main() {
    let x = Arc::new("foo".to_owned());
    let xs1 = (0..10).map(|_| x.clone()).collect::<Vec<_>>();
    let xs2 = (0..10).map(|_| x.clone()).collect::<Vec<_>>();
    // serialize the first batch
    let mut serializer = AllocSerializer::<256>::default();
    serializer.serialize_value(&xs1).unwrap();
    let (serializer, fallback, map) = serializer.into_components();
    let bytes1 = serializer.into_inner();
    // serialize the second batch, need to start with the original vec.
    let mut serializer =
        CompositeSerializer::new(AlignedSerializer::new(bytes1.clone()), fallback, map);
    serializer.serialize_value(&xs2).unwrap();
    let (serializer, _fallback, _map) = serializer.into_components();
    let bytes2 = serializer.into_inner();
    println!("bytes1");
    hexdump::hexdump(&bytes1);
    println!("bytes2");
    hexdump::hexdump(&bytes2);

    let archived = unsafe { archived_root::<Vec<Arc<String>>>(&bytes2[..]) };
    let mut d = SharedDeserializeMap::new();
    let unarchived: Vec<Arc<String>> = archived.deserialize(&mut d).unwrap();
    println!("{:?}", unarchived);

    let archived = check_archived_root::<Vec<Arc<String>>>(&bytes2[..]).unwrap();
    let mut d = SharedDeserializeMap::new();
    let unarchived: Vec<Arc<String>> = archived.deserialize(&mut d).unwrap();
    println!("{:?}", unarchived);
}
