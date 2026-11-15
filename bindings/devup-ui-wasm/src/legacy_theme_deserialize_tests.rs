    #[test]
    #[serial]
    fn deserialize_theme() {
        let _state = TestStateGuard::new();
        {
            let theme: Theme = serde_json::from_str(
                r##"{
            "colors": {
                "default": {
                    "primary": "#000"
                },
                "dark": {
                    "primary": "#fff"
                }
            },
            "typography": {
                "default": [
                    {
                        "fontFamily": "Arial",
                        "fontSize": "16px",
                        "fontWeight": 400,
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    },
                    {
                        "fontFamily": "Arial",
                        "fontSize": "24px",
                        "fontWeight": "400",
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    },
                    {
                        "fontFamily": "Arial",
                        "fontSize": "24px",
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    }
                ]
            }
        }"##,
            )
            .unwrap();
            assert_eq!(theme.breakpoints, vec![0, 480, 768, 992, 1280, 1600]);
            assert_debug_snapshot!(theme.to_css());
        }
        {
            let theme: Theme = serde_json::from_str(
                r##"{
            "colors": {
                "default": {
                    "primary": "#000"
                },
                "dark": {
                    "primary": "#fff"
                }
            },
            "typography": {
                "default":
                    {
                        "fontFamily": "Arial",
                        "fontSize": "16px",
                        "fontWeight": "400",
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    }
            }
        }"##,
            )
            .unwrap();
            assert_debug_snapshot!(theme);
        }

        {
            let theme: Theme = serde_json::from_str(
                r#"{
"typography":{"noticeButton":{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.02em"},"button":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"title":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"20px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"text":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"15px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"caption":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"12px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"14px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"noticeTitle":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"15px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"noticeText":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"14px","lineHeight":1.5,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"16px","lineHeight":1.5,"letterSpacing":"-0.02em"}],"h3":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"24px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"h1":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"28px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"36px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"body":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"20px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"noticeBold":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"14px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"notice":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"13px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"h2":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"20px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"28px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"result":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"24px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"32px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"resultPoint":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":800,"fontSize":"24px","lineHeight":1.4,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":800,"fontSize":"28px","lineHeight":1.4,"letterSpacing":"-0.01em"}],"resultText":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"18px","lineHeight":1.4,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"22px","lineHeight":1.4,"letterSpacing":"-0.01em"}],"resultList":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"16px","lineHeight":1.4,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"20px","lineHeight":1.4,"letterSpacing":"-0.01em"}]}
        }"#,
            )
            .unwrap();
            assert_debug_snapshot!(theme);
        }
    }
