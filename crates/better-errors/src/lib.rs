use frunk::{
    Coprod, Coproduct,
    coproduct::{
        CNil, CoprodInjector, CoprodUninjector, CoproductEmbedder, CoproductSelector,
        CoproductSubsetter, CoproductTaker,
    },
};

pub trait TypeSet {
    type Coprod: TypeSetCoprod;
}

pub trait TypeSetCoprod {
    type TypeSet: TypeSet;
    fn to_type_set(self) -> OneOf<Self::TypeSet>;
}

macro_rules! impl_one_of {
    ($(($($T:ident),+)),+ $(,)?) => {
        $(
            impl<$($T),+> TypeSet for ($($T,)+) {
                type Coprod = Coprod!($($T),+);
            }
            impl <$($T),+> TypeSetCoprod for Coprod!($($T),+) {
                type TypeSet = ($($T,)+);
                fn to_type_set(self) -> OneOf<Self::TypeSet> {
                    OneOf { value: self }
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

pub struct OneOf<T: TypeSet> {
    pub value: T::Coprod,
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

    pub fn convert<U>(self) -> U
    where
        T::Coprod: ConvertAs<U>,
    {
        self.value.convert()
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

pub trait ConvertAs<T> {
    fn convert(self) -> T;
}

impl<T> ConvertAs<T> for CNil {
    fn convert(self) -> T {
        unreachable!()
    }
}

impl<T, Head, Tail> ConvertAs<T> for Coproduct<Head, Tail>
where
    Head: Into<T>,
    Tail: ConvertAs<T>,
{
    fn convert(self) -> T {
        match self {
            Coproduct::Inl(h) => h.into(),
            Coproduct::Inr(t) => t.convert(),
        }
    }
}
