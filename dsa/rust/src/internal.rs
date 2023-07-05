use std::marker::PhantomData;

pub struct Assert<const T: bool> {
    __prevent_contstruction: PhantomData<()>,
}

impl const IsTrue for Assert<true> { }

#[const_trait]
pub trait IsTrue { }

impl const IsFalse for Assert<false> { }

#[const_trait]
pub trait IsFalse { }
