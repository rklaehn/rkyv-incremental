use std::{marker::PhantomData, sync::Arc};

use rkyv::{
    archived_root, check_archived_root,
    de::{deserializers::SharedDeserializeMap, SharedDeserializeRegistry},
    rc::{ArchivedRc, RcResolver},
    ser::{
        serializers::{AlignedSerializer, AllocSerializer, CompositeSerializer},
        ScratchSpace, Serializer, SharedSerializeRegistry,
    },
    vec::ArchivedVec,
    Archive, Archived, Deserialize, Resolver, Serialize,
};
struct Tree<'a> {
    prefix: String,
    value: Option<String>,
    children: Arc<Vec<Tree<'a>>>,
    _p: PhantomData<&'a ()>,
}

struct ArchivedTree<'a> {
    prefix: Archived<String>,
    value: Archived<Option<String>>,
    children: Archived<Arc<Vec<Tree<'a>>>>,
}

struct TreeResolver<'a> {
    prefix: Resolver<String>,
    value: Resolver<Option<String>>,
    children: Resolver<Arc<Vec<Tree<'a>>>>,
}

fn offset_from<T, U>(base: *const T, p: *const U) -> usize {
    let base = base as usize;
    let p = p as usize;
    assert!(p >= base);
    p - base
}

impl<'a> Archive for Tree<'a> {
    type Archived = ArchivedTree<'a>;

    type Resolver = TreeResolver<'a>;

    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        let TreeResolver {
            prefix,
            value,
            children,
            ..
        } = resolver;
        let ptr = &mut (*out).prefix;
        self.prefix
            .resolve(pos + offset_from(out, ptr), prefix, ptr);
        let ptr = &mut (*out).value;
        self.value.resolve(pos + offset_from(out, ptr), value, ptr);
        let ptr = &mut (*out).children;
        self.children
            .resolve(pos + offset_from(out, ptr), children, ptr);
    }
}

impl<'a, S> Serialize<S> for Tree<'a>
where
    S: Serializer + SharedSerializeRegistry + ScratchSpace,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        let prefix = self.prefix.serialize(serializer)?;
        let value = self.value.serialize(serializer)?;
        let children: &'static Arc<Vec<Tree<'static>>> =
            unsafe { std::mem::transmute(&self.children) };
        let children = children.serialize(serializer)?;
        Ok(TreeResolver {
            prefix,
            value,
            children,
        })
    }
}

impl<'a, D> Deserialize<Tree<'a>, D> for ArchivedTree<'a>
where
    D: SharedDeserializeRegistry + ?Sized,
{
    fn deserialize(&self, deserializer: &mut D) -> std::result::Result<Tree<'a>, D::Error> {
        let prefix: String = self.prefix.deserialize(deserializer)?;
        let value: Option<String> = self.value.deserialize(deserializer)?;
        let children: &'static ArchivedRc<ArchivedVec<ArchivedTree<'static>>, _> =
            unsafe { std::mem::transmute(&self.children) };
        let children: Arc<Vec<Tree<'a>>> = children.deserialize(deserializer)?;
        Ok(Tree {
            prefix,
            value,
            children,
            _p: PhantomData,
        })
    }
}

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
