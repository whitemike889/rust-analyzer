use super::*;

#[test]
fn macro_rules_are_globally_visible() {
    let map = def_map(
        "
        //- /lib.rs
        macro_rules! structs {
            ($($i:ident),*) => {
                $(struct $i { field: u32 } )*
            }
        }
        structs!(Foo);
        mod nested;

        //- /nested.rs
        structs!(Bar, Baz);
        ",
    );
    assert_snapshot!(map, @r###"
   ⋮crate
   ⋮Foo: t v
   ⋮nested: t
   ⋮
   ⋮crate::nested
   ⋮Bar: t v
   ⋮Baz: t v
    "###);
}

#[test]
fn macro_rules_can_define_modules() {
    let map = def_map(
        "
        //- /lib.rs
        macro_rules! m {
            ($name:ident) => { mod $name;  }
        }
        m!(n1);

        //- /n1.rs
        m!(n2)
        //- /n1/n2.rs
        struct X;
        ",
    );
    assert_snapshot!(map, @r###"
   ⋮crate
   ⋮n1: t
   ⋮
   ⋮crate::n1
   ⋮n2: t
   ⋮
   ⋮crate::n1::n2
   ⋮X: t v
    "###);
}

#[test]
fn macro_rules_from_other_crates_are_visible() {
    let map = def_map_with_crate_graph(
        "
        //- /main.rs
        foo::structs!(Foo, Bar)
        mod bar;

        //- /bar.rs
        use crate::*;

        //- /lib.rs
        #[macro_export]
        macro_rules! structs {
            ($($i:ident),*) => {
                $(struct $i { field: u32 } )*
            }
        }
        ",
        crate_graph! {
            "main": ("/main.rs", ["foo"]),
            "foo": ("/lib.rs", []),
        },
    );
    assert_snapshot!(map, @r###"
   ⋮crate
   ⋮Bar: t v
   ⋮Foo: t v
   ⋮bar: t
   ⋮
   ⋮crate::bar
   ⋮Bar: t v
   ⋮Foo: t v
   ⋮bar: t
    "###);
}

#[test]
fn unexpanded_macro_should_expand_by_fixedpoint_loop() {
    let map = def_map_with_crate_graph(
        "
        //- /main.rs
        macro_rules! baz {
            () => {
                use foo::bar;
            }
        }

        foo!();
        bar!();
        baz!();

        //- /lib.rs
        #[macro_export]
        macro_rules! foo {
            () => {
                struct Foo { field: u32 }
            }
        }
        #[macro_export]
        macro_rules! bar {
            () => {
                use foo::foo;
            }
        }
        ",
        crate_graph! {
            "main": ("/main.rs", ["foo"]),
            "foo": ("/lib.rs", []),
        },
    );
    assert_snapshot!(map, @r###"
   ⋮crate
   ⋮Foo: t v
   ⋮bar: m
   ⋮foo: m
    "###);
}

#[test]
fn macro_rules_from_other_crates_are_visible_with_macro_use() {
    covers!(macro_rules_from_other_crates_are_visible_with_macro_use);
    let map = def_map_with_crate_graph(
        "
        //- /main.rs
        structs!(Foo);
        structs_priv!(Bar);
        structs_not_exported!(MacroNotResolved1);
        crate::structs!(MacroNotResolved2);

        mod bar;

        #[macro_use]
        extern crate foo;

        //- /bar.rs
        structs!(Baz);
        crate::structs!(MacroNotResolved3);

        //- /lib.rs
        #[macro_export]
        macro_rules! structs {
            ($i:ident) => { struct $i; }
        }

        macro_rules! structs_not_exported {
            ($i:ident) => { struct $i; }
        }

        mod priv_mod {
            #[macro_export]
            macro_rules! structs_priv {
                ($i:ident) => { struct $i; }
            }
        }
        ",
        crate_graph! {
            "main": ("/main.rs", ["foo"]),
            "foo": ("/lib.rs", []),
        },
    );
    assert_snapshot!(map, @r###"
   ⋮crate
   ⋮Bar: t v
   ⋮Foo: t v
   ⋮bar: t
   ⋮foo: t
   ⋮
   ⋮crate::bar
   ⋮Baz: t v
    "###);
}

#[test]
fn prelude_is_macro_use() {
    covers!(prelude_is_macro_use);
    let map = def_map_with_crate_graph(
        "
        //- /main.rs
        structs!(Foo);
        structs_priv!(Bar);
        structs_outside!(Out);
        crate::structs!(MacroNotResolved2);

        mod bar;

        //- /bar.rs
        structs!(Baz);
        crate::structs!(MacroNotResolved3);

        //- /lib.rs
        #[prelude_import]
        use self::prelude::*;

        mod prelude {
            #[macro_export]
            macro_rules! structs {
                ($i:ident) => { struct $i; }
            }

            mod priv_mod {
                #[macro_export]
                macro_rules! structs_priv {
                    ($i:ident) => { struct $i; }
                }
            }
        }

        #[macro_export]
        macro_rules! structs_outside {
            ($i:ident) => { struct $i; }
        }
        ",
        crate_graph! {
            "main": ("/main.rs", ["foo"]),
            "foo": ("/lib.rs", []),
        },
    );
    assert_snapshot!(map, @r###"
   ⋮crate
   ⋮Bar: t v
   ⋮Foo: t v
   ⋮Out: t v
   ⋮bar: t
   ⋮
   ⋮crate::bar
   ⋮Baz: t v
    "###);
}

#[test]
fn prelude_cycle() {
    let map = def_map(
        "
        //- /lib.rs
        #[prelude_import]
        use self::prelude::*;

        declare_mod!();

        mod prelude {
            macro_rules! declare_mod {
                () => (mod foo {})
            }
        }
        ",
    );
    assert_snapshot!(map, @r###"
        ⋮crate
        ⋮prelude: t
        ⋮
        ⋮crate::prelude
    "###);
}

#[test]
fn plain_macros_are_legacy_textual_scoped() {
    let map = def_map(
        r#"
        //- /main.rs
        mod m1;
        bar!(NotFoundNotMacroUse);

        mod m2 {
            foo!(NotFoundBeforeInside2);
        }

        macro_rules! foo {
            ($x:ident) => { struct $x; }
        }
        foo!(Ok);

        mod m3;
        foo!(OkShadowStop);
        bar!(NotFoundMacroUseStop);

        #[macro_use]
        mod m5 {
            #[macro_use]
            mod m6 {
                macro_rules! foo {
                    ($x:ident) => { fn $x() {} }
                }
            }
        }
        foo!(ok_double_macro_use_shadow);

        baz!(NotFoundBefore);
        #[macro_use]
        mod m7 {
            macro_rules! baz {
                ($x:ident) => { struct $x; }
            }
        }
        baz!(OkAfter);

        //- /m1.rs
        foo!(NotFoundBeforeInside1);
        macro_rules! bar {
            ($x:ident) => { struct $x; }
        }

        //- /m3/mod.rs
        foo!(OkAfterInside);
        macro_rules! foo {
            ($x:ident) => { fn $x() {} }
        }
        foo!(ok_shadow);

        #[macro_use]
        mod m4;
        bar!(OkMacroUse);

        //- /m3/m4.rs
        foo!(ok_shadow_deep);
        macro_rules! bar {
            ($x:ident) => { struct $x; }
        }
        "#,
    );
    assert_snapshot!(map, @r###"
   ⋮crate
   ⋮Ok: t v
   ⋮OkAfter: t v
   ⋮OkShadowStop: t v
   ⋮m1: t
   ⋮m2: t
   ⋮m3: t
   ⋮m5: t
   ⋮m7: t
   ⋮ok_double_macro_use_shadow: v
   ⋮
   ⋮crate::m7
   ⋮
   ⋮crate::m1
   ⋮
   ⋮crate::m5
   ⋮m6: t
   ⋮
   ⋮crate::m5::m6
   ⋮
   ⋮crate::m2
   ⋮
   ⋮crate::m3
   ⋮OkAfterInside: t v
   ⋮OkMacroUse: t v
   ⋮m4: t
   ⋮ok_shadow: v
   ⋮
   ⋮crate::m3::m4
   ⋮ok_shadow_deep: v
    "###);
}

#[test]
fn type_value_macro_live_in_different_scopes() {
    let map = def_map(
        "
        //- /main.rs
        #[macro_export]
        macro_rules! foo {
            ($x:ident) => { type $x = (); }
        }

        foo!(foo);
        use foo as bar;

        use self::foo as baz;
        fn baz() {}
        ",
    );
    assert_snapshot!(map, @r###"
        ⋮crate
        ⋮bar: t m
        ⋮baz: t v m
        ⋮foo: t m
    "###);
}

#[test]
fn macro_use_can_be_aliased() {
    let map = def_map_with_crate_graph(
        "
        //- /main.rs
        #[macro_use]
        extern crate foo;

        foo!(Direct);
        bar!(Alias);

        //- /lib.rs
        use crate::foo as bar;

        mod m {
            #[macro_export]
            macro_rules! foo {
                ($x:ident) => { struct $x; }
            }
        }
        ",
        crate_graph! {
            "main": ("/main.rs", ["foo"]),
            "foo": ("/lib.rs", []),
        },
    );
    assert_snapshot!(map, @r###"
        ⋮crate
        ⋮Alias: t v
        ⋮Direct: t v
        ⋮foo: t
    "###);
}

#[test]
fn path_quantified_macros() {
    let map = def_map(
        "
        //- /main.rs
        macro_rules! foo {
            ($x:ident) => { struct $x; }
        }

        crate::foo!(NotResolved);

        crate::bar!(OkCrate);
        bar!(OkPlain);
        alias1!(NotHere);
        m::alias1!(OkAliasPlain);
        m::alias2!(OkAliasSuper);
        m::alias3!(OkAliasCrate);
        not_found!(NotFound);

        mod m {
            #[macro_export]
            macro_rules! bar {
                ($x:ident) => { struct $x; }
            }

            pub use bar as alias1;
            pub use super::bar as alias2;
            pub use crate::bar as alias3;
            pub use self::bar as not_found;
        }
        ",
    );
    assert_snapshot!(map, @r###"
        ⋮crate
        ⋮OkAliasCrate: t v
        ⋮OkAliasPlain: t v
        ⋮OkAliasSuper: t v
        ⋮OkCrate: t v
        ⋮OkPlain: t v
        ⋮bar: m
        ⋮m: t
        ⋮
        ⋮crate::m
        ⋮alias1: m
        ⋮alias2: m
        ⋮alias3: m
        ⋮not_found: _
    "###);
}
