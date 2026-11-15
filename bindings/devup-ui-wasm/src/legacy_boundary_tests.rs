    #[rstest]
    #[case(Ok(42), false, None, Ok(42))]
    #[case(Err("extract failed"), false, None, Err("extract failed"))]
    #[case(Ok(42), true, None, Ok(42))]
    #[case(Ok(42), true, Some("resolver failed"), Err("resolver failed"))]
    #[case(
        Err("extract failed"),
        true,
        Some("resolver failed"),
        Err("resolver failed")
    )]
    fn boundary_fault_takes_precedence_when_resolver_failed(
        #[case] extracted: Result<u8, &str>,
        #[case] has_resolver: bool,
        #[case] message: Option<&str>,
        #[case] expected: Result<u8, &str>,
    ) {
        let fault = has_resolver.then(|| RefCell::new(message.map(str::to_string)));
        let result = checked_extraction(extracted, fault.as_ref());
        assert_eq!(result, expected.map_err(str::to_string));
    }

    #[test]
    #[serial]
    fn boundary_sheet_stays_unchanged_when_resolver_failed() {
        reset_build_state_internal();
        let before = export_sheet_internal().unwrap();
        let fault = RefCell::new(Some("resolver failed".to_string()));
        let result = code_extract_internal_impl(
            "boundary.tsx",
            "import { Box } from '@devup-ui/react'; export const view = <Box bg='red' />;",
            "@devup-ui/react",
            "df".to_string(),
            true,
            false,
            false,
            HashMap::new(),
            SourceMapMode::Skip,
            None,
            Some(&fault),
            None,
            &[],
        );
        assert_eq!(result.err(), Some("resolver failed".to_string()));
        assert_eq!(export_sheet_internal().unwrap(), before);
        reset_build_state_internal();
    }
