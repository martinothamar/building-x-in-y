#![allow(dead_code)]

pub(crate) struct AssertPow2<const A: usize>;

impl<const A: usize> AssertPow2<A>
where
    AssertPow2<A>: Sized,
{
    pub const A: () = assert!((A & (A - 1)) == 0, "\nSize must be a power of two");
}

pub(crate) type Align<const N: usize> = <USize<N> as Alignment>::Archetype;

pub(crate) struct USize<const N: usize>
where
    Self: Alignment;

#[const_trait]
pub(crate) trait Alignment {
    type Archetype;
}

#[repr(C, align(64))]
pub(crate) struct Alignment64;
impl const Alignment for USize<64> {
    type Archetype = Alignment64;
}
#[repr(C, align(128))]
pub(crate) struct Alignment128;
impl const Alignment for USize<128> {
    type Archetype = Alignment128;
}
#[repr(C, align(256))]
pub(crate) struct Alignment256;
impl const Alignment for USize<256> {
    type Archetype = Alignment256;
}
#[repr(C, align(512))]
pub(crate) struct Alignment512;
impl const Alignment for USize<512> {
    type Archetype = Alignment512;
}
#[repr(C, align(1024))]
pub(crate) struct Alignment1024;
impl const Alignment for USize<1024> {
    type Archetype = Alignment1024;
}
#[repr(C, align(2048))]
pub(crate) struct Alignment2048;
impl const Alignment for USize<2048> {
    type Archetype = Alignment2048;
}
#[repr(C, align(4096))]
pub(crate) struct Alignment4096;
impl const Alignment for USize<4096> {
    type Archetype = Alignment4096;
}
#[repr(C, align(8192))]
pub(crate) struct Alignment8192;
impl const Alignment for USize<8192> {
    type Archetype = Alignment8192;
}
#[repr(C, align(16384))]
pub(crate) struct Alignment16384;
impl const Alignment for USize<16384> {
    type Archetype = Alignment16384;
}
#[repr(C, align(32768))]
pub(crate) struct Alignment32768;
impl const Alignment for USize<32768> {
    type Archetype = Alignment32768;
}
#[repr(C, align(65536))]
pub(crate) struct Alignment65536;
impl const Alignment for USize<65536> {
    type Archetype = Alignment65536;
}
#[repr(C, align(131072))]
pub(crate) struct Alignment131072;
impl const Alignment for USize<131072> {
    type Archetype = Alignment131072;
}
#[repr(C, align(262144))]
pub(crate) struct Alignment262144;
impl const Alignment for USize<262144> {
    type Archetype = Alignment262144;
}
#[repr(C, align(524288))]
pub(crate) struct Alignment524288;
impl const Alignment for USize<524288> {
    type Archetype = Alignment524288;
}
#[repr(C, align(1048576))]
pub(crate) struct Alignment1048576;
impl const Alignment for USize<1048576> {
    type Archetype = Alignment1048576;
}
#[repr(C, align(2097152))]
pub(crate) struct Alignment2097152;
impl const Alignment for USize<2097152> {
    type Archetype = Alignment2097152;
}
#[repr(C, align(4194304))]
pub(crate) struct Alignment4194304;
impl const Alignment for USize<4194304> {
    type Archetype = Alignment4194304;
}
#[repr(C, align(8388608))]
pub(crate) struct Alignment8388608;
impl const Alignment for USize<8388608> {
    type Archetype = Alignment8388608;
}
#[repr(C, align(16777216))]
pub(crate) struct Alignment16777216;
impl const Alignment for USize<16777216> {
    type Archetype = Alignment16777216;
}
#[repr(C, align(33554432))]
pub(crate) struct Alignment33554432;
impl const Alignment for USize<33554432> {
    type Archetype = Alignment33554432;
}
#[repr(C, align(67108864))]
pub(crate) struct Alignment67108864;
impl const Alignment for USize<67108864> {
    type Archetype = Alignment67108864;
}
#[repr(C, align(134217728))]
pub(crate) struct Alignment134217728;
impl const Alignment for USize<134217728> {
    type Archetype = Alignment134217728;
}
#[repr(C, align(268435456))]
pub(crate) struct Alignment268435456;
impl const Alignment for USize<268435456> {
    type Archetype = Alignment268435456;
}
