use argh::FromArgs;

#[derive(FromArgs)]
/// Reach new heights.
pub struct FenderArgs {
    /// the url which contains a list of bikes of a given brand
    #[argh(positional, short = 'u')]
    pub bike_url: String,
    /// the type of bike, e.g., aeroad, roadlite
    #[argh(positional, short = 't')]
    pub bike_type: String
}
