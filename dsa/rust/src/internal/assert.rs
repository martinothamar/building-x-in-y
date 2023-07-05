use std::marker::PhantomData;

pub(crate) struct Assert<const T: bool> {
    __prevent_contstruction: PhantomData<()>,
}

impl const IsTrue for Assert<true> {}

#[const_trait]
pub(crate) trait IsTrue {}

impl const IsFalse for Assert<false> {}

#[const_trait]
pub(crate) trait IsFalse {}
