pub struct Biome{

}

#[derive(Copy,Clone,PartialEq,Eq)]
pub struct BiomeId(u32);

impl BiomeId{
    pub fn init()->Self{
        BiomeId(u32::max_value())
    }
}

pub struct BiomeRegistry{
}

impl BiomeRegistry{
    pub fn new()->Self{
        unimplemented!()
    }

    pub fn register(&mut self,b:Biome)->BiomeId{
        unimplemented!();
    }

    pub fn list(&self)->&[BiomeId]{
        unimplemented!();
    }
}

