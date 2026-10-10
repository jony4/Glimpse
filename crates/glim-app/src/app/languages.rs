//! Fill grammar resources omitted by the pinned toolkit, without replacing its parsers.
use gpui_kit::component::highlighter::LanguageRegistry;

pub fn init() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let registry = LanguageRegistry::singleton();
        for (name, highlights) in [
            ("json", include_str!("../../../../assets/highlights/json.scm")),
            ("csharp", include_str!("../../../../assets/highlights/csharp.scm")),
            ("swift", include_str!("../../../../assets/highlights/swift.scm")),
            ("graphql", include_str!("../../../../assets/highlights/graphql.scm")),
            ("proto", include_str!("../../../../assets/highlights/proto.scm")),
            ("cmake", include_str!("../../../../assets/highlights/cmake.scm")),
        ] {
            if let Some(mut config) = registry.language(name) {
                config.highlights = highlights.to_owned().into();
                registry.register(name, &config);
            }
        }
        // The toolkit currently uses JavaScript injections for both EJS and ERB.
        if let Some(mut config) = registry.language("erb") {
            config.injections = r#"
                ((content) @injection.content (#set! injection.language "html") (#set! injection.combined))
                ((code) @injection.content (#set! injection.language "ruby") (#set! injection.combined))
            "#.to_owned().into();
            config.injection_languages = vec!["html".into(), "ruby".into()];
            registry.register("erb", &config);
        }
    });
}
