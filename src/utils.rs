pub trait VecExt<T> {
    fn map_into<R>(self) -> Vec<R>
    where
        R: From<T>;
}

impl<T> VecExt<T> for Vec<T> {
    fn map_into<R>(self) -> Vec<R>
    where
        R: From<T>,
    {
        self.into_iter().map(Into::into).collect()
    }
}
