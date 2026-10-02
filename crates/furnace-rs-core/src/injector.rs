//! Typed construction inputs and asynchronous application provider constructors.

use crate::{
    ConstructionContext, DependencyDescriptor, LifecycleProviderFuture, LifecycleResource,
    ProviderContribution, ProviderDescriptor, ProviderFuture, ProviderKind, ProviderVisibility,
    Result, SourceLocation,
};
use std::{
    any::{TypeId, type_name},
    future::Future,
    marker::PhantomData,
    sync::Arc,
};

/// Constructs an application output from explicitly declared dependencies.
///
/// Managed `#[burner]` and `#[storage]` providers implement this automatically.
/// Plain structs can implement it directly; `Injector<Output>` binds a separate
/// constructor type to an output such as `Arc<dyn Trait>`.
pub trait Injector<T = Self>: Sized + Send + Sync + 'static
where
    T: Send + Sync + 'static,
{
    /// Unit or a tuple of up to sixteen cloned application dependencies.
    type Dependencies: InjectionDependencies;

    /// Constructs the native output after its dependencies are available.
    fn inject(dependencies: Self::Dependencies) -> impl Future<Output = Result<T>> + Send;

    /// Attaches resource lifecycle hooks to the constructed output.
    fn lifecycle(value: T) -> LifecycleResource<T> {
        LifecycleResource::new(value)
    }

    /// Static constructor metadata; managed providers retain their richer metadata.
    #[doc(hidden)]
    fn descriptor() -> &'static ProviderDescriptor {
        injector_descriptor::<T, Self>()
    }
}

/// Declared construction inputs: `()` or a tuple with one through sixteen elements.
///
/// Each element must be `Clone + Send + Sync + 'static`. A single dependency
/// uses `(Dependency,)`. Resolution clones provider values, so shared handles
/// retain their application instance. Implementations are sealed.
pub trait InjectionDependencies: sealed::Dependencies + Sized + Send + Sync + 'static {
    /// Describes input types without resolving or constructing their values.
    fn descriptors() -> &'static [DependencyDescriptor] {
        Self::DESCRIPTORS
    }

    /// Resolves already constructed inputs in their declared order.
    fn resolve(context: &ConstructionContext<'_>) -> Result<Self>;
}

mod sealed {
    use crate::DependencyDescriptor;
    pub trait Dependencies {
        const DESCRIPTORS: &'static [DependencyDescriptor];
    }
}

impl sealed::Dependencies for () {
    const DESCRIPTORS: &'static [DependencyDescriptor] = &[];
}
impl InjectionDependencies for () {
    fn resolve(_: &ConstructionContext<'_>) -> Result<Self> {
        Ok(())
    }
}

macro_rules! dependency_tuple {
    ($($ty:ident),+) => {
        impl<$($ty: Clone + Send + Sync + 'static),+> sealed::Dependencies for ($($ty,)+) {
            const DESCRIPTORS: &'static [DependencyDescriptor] = &[$(
                DependencyDescriptor::new(stringify!($ty), TypeId::of::<$ty>)
                    .with_runtime_type_name(type_name::<$ty>),
            )+];
        }
        impl<$($ty: Clone + Send + Sync + 'static),+> InjectionDependencies for ($($ty,)+) {
            #[allow(clippy::result_large_err)]
            fn resolve(context: &ConstructionContext<'_>) -> Result<Self> {
                Ok(($(context.resolve::<$ty>()?.as_ref().clone(),)+))
            }
        }
    };
}
dependency_tuple!(A);
dependency_tuple!(A, B);
dependency_tuple!(A, B, C);
dependency_tuple!(A, B, C, D);
dependency_tuple!(A, B, C, D, E);
dependency_tuple!(A, B, C, D, E, F);
dependency_tuple!(A, B, C, D, E, F, G);
dependency_tuple!(A, B, C, D, E, F, G, H);
dependency_tuple!(A, B, C, D, E, F, G, H, J);
dependency_tuple!(A, B, C, D, E, F, G, H, J, K);
dependency_tuple!(A, B, C, D, E, F, G, H, J, K, L);
dependency_tuple!(A, B, C, D, E, F, G, H, J, K, L, M);
dependency_tuple!(A, B, C, D, E, F, G, H, J, K, L, M, N);
dependency_tuple!(A, B, C, D, E, F, G, H, J, K, L, M, N, O);
dependency_tuple!(A, B, C, D, E, F, G, H, J, K, L, M, N, O, P);
dependency_tuple!(A, B, C, D, E, F, G, H, J, K, L, M, N, O, P, Q);

fn construct<'a, T, I>(context: &'a ConstructionContext<'a>) -> ProviderFuture<'a>
where
    T: Send + Sync + 'static,
    I: Injector<T>,
{
    Box::pin(async move {
        let value = I::inject(I::Dependencies::resolve(context)?).await?;
        Ok(Arc::new(value) as crate::ErasedProvider)
    })
}

fn construct_resource<'a, T, I>(context: &'a ConstructionContext<'a>) -> LifecycleProviderFuture<'a>
where
    T: Send + Sync + 'static,
    I: Injector<T>,
{
    Box::pin(async move {
        let value = I::inject(I::Dependencies::resolve(context)?).await?;
        Ok(ProviderContribution::from_resource(I::lifecycle(value)))
    })
}

/// Static descriptor bridge for deliberately discoverable native integrations.
#[doc(hidden)]
pub struct InjectorMetadata<T, I>(PhantomData<fn() -> (T, I)>);
impl<T: Send + Sync + 'static, I: Injector<T>> InjectorMetadata<T, I> {
    /// Constructor metadata suitable for an inventory submission.
    pub const DESCRIPTOR: ProviderDescriptor = ProviderDescriptor::new(
        ProviderKind::Provider,
        "injector output",
        TypeId::of::<T>,
        <I::Dependencies as sealed::Dependencies>::DESCRIPTORS,
        ProviderVisibility::Private,
        SourceLocation::new(file!(), line!(), column!()),
        construct::<T, I>,
    )
    .with_resolved_type_name(type_name::<T>)
    .with_runtime_type_name(type_name::<T>)
    .with_lifecycle_constructor(construct_resource::<T, I>);
}

/// Returns statically promoted metadata without constructing any dependencies.
#[doc(hidden)]
pub fn injector_descriptor<T: Send + Sync + 'static, I: Injector<T>>() -> &'static ProviderDescriptor
{
    &InjectorMetadata::<T, I>::DESCRIPTOR
}
