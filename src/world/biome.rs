pub struct Biome{

}

pub struct BiomeId(u32);

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

