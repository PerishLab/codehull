mod acceptance {
    mod estate;
    mod git;
    mod ground;
    mod haul;
    mod issuer;
    mod seam;
    mod stage;
    mod wire;

    fn exe() -> &'static std::path::Path {
        std::path::Path::new(env!("CARGO_BIN_EXE_codehull-api"))
    }

    fn open() -> (stage::Stage, wire::Wire) {
        let stage = stage::Stage::live(exe());
        let wire = wire::Wire::new(&stage.base);
        (stage, wire)
    }
}
