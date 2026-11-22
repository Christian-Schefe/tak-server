use frunk::{
    Coprod, Coproduct, HList,
    coproduct::{
        CNil, CoprodInjector, CoprodUninjector, CoproductEmbedder, CoproductSelector,
        CoproductSubsetter, CoproductTaker,
    },
};

pub trait TypeSet {
    type Coprod: TypeSetCoprod;
    type Product: TypeSetProduct;
    fn all(tuple: Self) -> AllOf<Self>
    where
        Self: Sized;
}

pub trait TypeSetCoprod {
    type TypeSet: TypeSet;
    fn to_type_set(self) -> OneOf<Self::TypeSet>;
}

pub trait TypeSetProduct {
    type TypeSet: TypeSet;
    fn to_type_set(self) -> AllOf<Self::TypeSet>;
}

macro_rules! impl_one_of {
    ($(($($T:ident),+)),+ $(,)?) => {
        $(
            impl<$($T),+> TypeSet for ($($T,)+) {
                type Coprod = Coprod!($($T),+);
                type Product = HList!($($T),+);
                fn all(tuple: Self) -> AllOf<Self> {
                    AllOf { value: tuple.into() }
                }
            }
            impl <$($T),+> TypeSetCoprod for Coprod!($($T),+) {
                type TypeSet = ($($T,)+);
                fn to_type_set(self) -> OneOf<Self::TypeSet> {
                    OneOf { value: self }
                }
            }
            impl <$($T),+> TypeSetProduct for HList!($($T),+) {
                type TypeSet = ($($T,)+);
                fn to_type_set(self) -> AllOf<Self::TypeSet> {
                    AllOf { value: self }
                }
            }
        )+
    };
}

impl_one_of! {
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G),
    (A, B, C, D, E, F, G, H),
    (A, B, C, D, E, F, G, H, I),
    (A, B, C, D, E, F, G, H, I, J),
    (A, B, C, D, E, F, G, H, I, J, K),
    (A, B, C, D, E, F, G, H, I, J, K, L),
    (A, B, C, D, E, F, G, H, I, J, K, L, M),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X),
}

pub struct AllOf<T: TypeSet> {
    value: T::Product,
}

impl<T: TypeSet> AllOf<T> {
    pub fn new(value: T) -> AllOf<T> {
        T::all(value)
    }
}

pub struct OneOf<T: TypeSet> {
    value: T::Coprod,
}

impl<T: TypeSet> OneOf<T> {
    pub fn new<V, I>(value: V) -> Self
    where
        T::Coprod: CoprodInjector<V, I>,
    {
        Self {
            value: T::Coprod::inject(value),
        }
    }

    pub fn broaden<U, I>(self) -> OneOf<U>
    where
        U: TypeSet,
        T::Coprod: CoproductEmbedder<U::Coprod, I>,
    {
        OneOf {
            value: self.value.embed(),
        }
    }

    pub fn get<E, I>(&self) -> Option<&E>
    where
        T::Coprod: CoproductSelector<E, I>,
    {
        self.value.get()
    }

    pub fn take<E, I>(self) -> Option<E>
    where
        T::Coprod: CoproductTaker<E, I>,
    {
        self.value.take()
    }

    pub fn narrow<E, I>(self) -> Result<E, OneOf<<Self as OneOfNarrow<E, I>>::Remainder>>
    where
        Self: OneOfNarrow<E, I>,
    {
        OneOfNarrow::narrow(self)
    }

    pub fn split<E, I>(self) -> Result<OneOf<E>, OneOf<<Self as OneOfSplit<E, I>>::Remainder>>
    where
        E: TypeSet,
        Self: OneOfSplit<E, I>,
    {
        OneOfSplit::split(self)
    }

    pub fn erase_as<E: ?Sized>(&self) -> &E
    where
        T: EraseAs<E> + FoldAs<E, fn(&E) -> &E, &E>,
    {
        FoldAs::fold_as(&self.value, &mut |x| x.erase_as())
    }
}

impl<T: TypeSet> Clone for OneOf<T>
where
    T::Coprod: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

impl<T: TypeSet> PartialEq for OneOf<T>
where
    T::Coprod: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: TypeSet> std::fmt::Debug for OneOf<T>
where
    T::Coprod: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OneOf({:?})", self.value)
    }
}

pub trait OneOfNarrow<E, I> {
    type Remainder: TypeSet;
    fn narrow(self) -> Result<E, OneOf<Self::Remainder>>;
}

impl<T: TypeSet, E, I> OneOfNarrow<E, I> for OneOf<T>
where
    T::Coprod: CoprodUninjector<E, I>,
    <T::Coprod as CoprodUninjector<E, I>>::Remainder: TypeSetCoprod,
{
    type Remainder = <<T::Coprod as CoprodUninjector<E, I>>::Remainder as TypeSetCoprod>::TypeSet;

    fn narrow(self) -> Result<E, OneOf<Self::Remainder>> {
        match self.value.uninject() {
            Ok(e) => Ok(e),
            Err(remainder) => Err(remainder.to_type_set()),
        }
    }
}

pub trait OneOfSplit<E: TypeSet, I> {
    type Remainder: TypeSet;
    fn split(self) -> Result<OneOf<E>, OneOf<Self::Remainder>>;
}

impl<T: TypeSet, E: TypeSet, I> OneOfSplit<E, I> for OneOf<T>
where
    T::Coprod: CoproductSubsetter<E::Coprod, I>,
    <T::Coprod as CoproductSubsetter<E::Coprod, I>>::Remainder: TypeSetCoprod,
{
    type Remainder =
        <<T::Coprod as CoproductSubsetter<E::Coprod, I>>::Remainder as TypeSetCoprod>::TypeSet;

    fn split(self) -> Result<OneOf<E>, OneOf<Self::Remainder>> {
        match self.value.subset() {
            Ok(e) => Ok(OneOf { value: e }),
            Err(remainder) => Err(remainder.to_type_set()),
        }
    }
}

pub trait EraseAs<T: ?Sized> {
    fn erase_as(&self) -> &T;
}
pub trait FoldAs<T: ?Sized, F, R> {
    fn fold_as(&self, f: &mut F) -> R;
}

impl<T: ?Sized, F, R> FoldAs<T, F, R> for CNil {
    fn fold_as(&self, _: &mut F) -> R {
        match *self {}
    }
}

impl<Head, Tail, T: ?Sized, F, R> FoldAs<T, F, R> for Coproduct<Head, Tail>
where
    Head: EraseAs<T>,
    Tail: FoldAs<T, F, R>,
    F: FnMut(&T) -> R,
{
    fn fold_as(&self, f: &mut F) -> R {
        match self {
            Coproduct::Inl(value) => f(value.erase_as()),
            Coproduct::Inr(tail) => tail.fold_as(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_of() {
        let one_of = OneOf::<(i32, String)>::new(42);
        let broadened = one_of.clone().broaden::<(i32, String, bool), _>();
        let broadened2 = one_of.clone().broaden::<(i32, bool, String), _>();
        let taken: Option<i32> = one_of.clone().take();
        let narrowed = one_of.clone().narrow::<i32, _>();
        let subset = broadened.clone().split::<(i32,), _>();
        let subset2 = broadened.clone().split::<(bool, String), _>();

        assert_eq!(one_of, one_of);
        assert_eq!(broadened, broadened);
        assert_eq!(broadened2, broadened2);
    }

    fn test_broaden_return() {
        fn inner(
            one_of: Result<(), OneOf<(i32, String)>>,
        ) -> Result<(), OneOf<(i32, String, bool)>> {
            one_of.map_err(OneOf::broaden)?;
            Ok(())
        }
    }

    struct Obj1;
    struct Obj2;
    struct Obj3;
    trait ObjTrait {}
    impl ObjTrait for Obj1 {}
    impl ObjTrait for Obj2 {}
    impl ObjTrait for Obj3 {}

    impl<T: ObjTrait> EraseAs<dyn ObjTrait> for T {
        fn erase_as(&self) -> &(dyn ObjTrait + 'static) {
            todo!()
        }
    }

    fn test_erase() {
        let one_of = OneOf::<(i32, String)>::new(42);

        let erased: &dyn ObjTrait = one_of.erase_as();
    }
}
