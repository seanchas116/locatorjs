use swc_common::sync::OnceCell;
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_common::{LineCol, Spanned};
use swc_core::common::SyntaxContext;
use swc_core::ecma::ast::*;
use swc_core::plugin::plugin_transform;
use swc_core::plugin::proxies::{PluginSourceMapProxy, TransformPluginProgramMetadata};
use swc_core::{ecma::ast::JSXElement, ecma::transforms::testing::test, ecma::visit::*};
use swc_ecma_parser::{Syntax, TsConfig};
use swc_ecma_visit::{as_folder, noop_visit_mut_type, Fold, VisitMut};

struct LineColMapping {
    line_cols: Vec<LineCol>,
}

impl LineColMapping {
    pub fn new(source: &str) -> Self {
        let mut line_cols = Vec::new();
        let mut line = 1;
        let mut col = 0;
        for c in source.chars() {
            line_cols.push(LineCol {
                line: line,
                col: col,
            });
            if c == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        line_cols.push(LineCol {
            line: line,
            col: col,
        });
        LineColMapping {
            line_cols: line_cols,
        }
    }
}

struct AddLocatorVisitor {
    line_col_mapping: LineColMapping,
    start_pos: u32,
}

impl AddLocatorVisitor {
    pub fn new(line_col_mapping: LineColMapping) -> Self {
        AddLocatorVisitor {
            line_col_mapping,
            start_pos: 0,
        }
    }
}

impl VisitMut for AddLocatorVisitor {
    fn visit_mut_program(&mut self, n: &mut Program) {
        self.start_pos = n.span().lo.0;
        n.visit_mut_children_with(self);
    }

    fn visit_mut_jsx_element(&mut self, jsx: &mut JSXElement) {
        let mut opening = jsx.opening.clone();
        let start = opening.span.lo.0 - self.start_pos;

        let linecol = self.line_col_mapping.line_cols[start as usize];
        println!("linecol: {:?}", linecol);

        let loc = format!("{}:{}:{}", "file", linecol.line, linecol.col);

        opening.attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
            span: opening.span,
            name: JSXAttrName::Ident(Ident::new("dataLocator".into(), opening.span)),
            value: Some(JSXAttrValue::Lit(Lit::Str(
                Str {
                    span: opening.span,
                    value: loc.clone().into(),
                    raw: None,
                }
                .into(),
            ))),
        }));

        // Assigning modified opening back to jsx object.
        jsx.opening = opening;
    }
}

#[plugin_transform]
pub fn process(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let src = metadata.source_map.source_file.get().unwrap().src.clone();
    let line_col_mapping = LineColMapping::new(&src);
    program.fold_with(&mut as_folder(AddLocatorVisitor::new(line_col_mapping)))
}

test!(
    Syntax::Typescript(TsConfig {
        tsx: true,
        ..Default::default()
    }),
    |_| {
        as_folder(AddLocatorVisitor::new(LineColMapping::new(
            "const x = <div />;",
        )))
    },
    test_one,
    r#"const x = <div/>;"#,
    r#"const x = <div dataLocator="file:1:10"/>;"#
);
