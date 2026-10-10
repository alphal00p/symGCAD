use symgcad::corpus::{MAX_LITERAL_FIXTURE_BYTES, parse_literal_symanzik};

const SOURCE: &str = "U = ExponentiatedPolynomial(expolist=[[1,0,0],[0,1,0]], coeffs=[1,1], polysymbols=['x0','x1','eps'], exponent='-2')\nF = ExponentiatedPolynomial(expolist=[[2,0,0],[0,0,0]], coeffs=['mt2','-s'], polysymbols=['x0','x1','eps'], exponent='-1')";

#[test]
fn named_literal_parser_preserves_expression_and_inactive_map() {
    let u = parse_literal_symanzik(SOURCE, "U").unwrap();
    let f = parse_literal_symanzik(SOURCE, "F").unwrap();
    assert_eq!(u.expression, "(1)*x0 + (1)*x1");
    assert_eq!(f.expression, "(mt2)*x0^2 + (-s)");
    assert_eq!(f.variables, ["x0", "x1"]);
    assert_eq!(u.variables, f.variables);
    assert_eq!(f.exponents, vec![vec![2, 0, 0], vec![0, 0, 0]]);
}

#[test]
fn rejects_executable_trailing_duplicate_and_malformed_data() {
    for text in [
        SOURCE.replace("expolist=[[1,0,0],[0,1,0]]", "expolist=__import__('os')"),
        format!("{} + dangerous()", SOURCE),
        format!("{SOURCE}\n{}", SOURCE.lines().last().unwrap()),
        SOURCE.replace(
            "polysymbols=['x0','x1','eps']",
            "polysymbols=['x0','x0','eps']",
        ),
        SOURCE.replace("[[2,0,0],[0,0,0]]", "[[2,0],[0,0,0]]"),
        SOURCE.replace("[[2,0,0],[0,0,0]]", "[[2,0,1],[0,0,0]]"),
        SOURCE.replace("coeffs=['mt2','-s']", "coeffs=['mt2']"),
    ] {
        assert!(
            parse_literal_symanzik(&text, "F").is_err()
                || parse_literal_symanzik(&text, "U").is_err()
        );
    }
    assert!(parse_literal_symanzik("li=LoopIntegralFromPropagators()", "F").is_err());
}

#[test]
fn rejects_size_depth_degree_and_symbol_caps() {
    assert!(parse_literal_symanzik(&" ".repeat(MAX_LITERAL_FIXTURE_BYTES + 1), "F").is_err());
    assert!(
        parse_literal_symanzik(
            &SOURCE.replace("[[2,0,0],[0,0,0]]", "[[33,0,0],[0,0,0]]"),
            "F"
        )
        .is_err()
    );
    let nested = format!(
        "F = ExponentiatedPolynomial(expolist={}0{}, coeffs=[], polysymbols=[])",
        "[".repeat(32),
        "]".repeat(32)
    );
    assert!(parse_literal_symanzik(&nested, "F").is_err());
    assert!(parse_literal_symanzik(SOURCE, "p").is_err());
}
