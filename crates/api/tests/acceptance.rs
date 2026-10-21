mod acceptance {
    mod bench;
    mod estate;
    mod git;
    mod ground;
    mod haul;
    mod issuer;
    mod seam;
    mod wire;

    fn open() -> (bench::Bench, wire::Wire) {
        let bench = bench::Bench::new();
        let wire = bench.wire();
        (bench, wire)
    }
}
