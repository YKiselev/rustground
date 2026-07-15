use crate::world::HyperCube;

pub trait WorldRenderer {
    type Context<'a>: WorldRendererContext;

    fn draw_world<H>(&mut self, handler: H)
    where
        H: FnMut(&mut Self::Context<'_>);
}

pub trait WorldRendererContext {
    fn draw_hyper_cube(&mut self, cube: &HyperCube);
}
