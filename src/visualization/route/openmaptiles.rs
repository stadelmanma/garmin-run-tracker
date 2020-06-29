//! Use an instance of open map tiles to draw a course route
use crate::Location;
use super::RouteDrawingService;

pub struct OpenMapTiles {

}

impl RouteDrawingService for OpenMapTiles {
    fn draw_route(&self, trace: &[Location]) -> Result<(), Box<dyn std::error::Error>> {
        todo!();
    }
}
