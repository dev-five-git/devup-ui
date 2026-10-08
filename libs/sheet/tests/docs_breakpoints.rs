use sheet::theme::Theme;
use std::{
    error::Error,
    io::{Error as IoError, ErrorKind},
};

#[test]
fn typography_breakpoint_indices_match_default_theme() -> Result<(), Box<dyn Error>> {
    let docs =
        include_str!("../../../apps/landing/src/app/(detail)/docs/devup/typography/page.mdx");
    let defaults = Theme::default().breakpoints;

    let documented: Vec<(usize, u16)> = docs
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- Index "))
        .map(|entry| -> Result<_, Box<dyn Error>> {
            let (index, value) = entry.split_once(':').ok_or_else(|| {
                IoError::new(
                    ErrorKind::InvalidData,
                    "typography breakpoint entry must contain an index and value",
                )
            })?;
            let pixels = value
                .trim()
                .strip_prefix('`')
                .and_then(|value| value.split_once("px`").map(|(pixels, _)| pixels))
                .ok_or_else(|| {
                    IoError::new(
                        ErrorKind::InvalidData,
                        "typography breakpoint value must be pixels in inline code",
                    )
                })?;
            Ok((index.parse()?, pixels.parse()?))
        })
        .collect::<Result<_, _>>()?;

    assert_eq!(
        documented,
        defaults.into_iter().enumerate().collect::<Vec<_>>(),
        "public typography breakpoint indices must match Theme::default()"
    );
    Ok(())
}

#[test]
fn documented_breakpoint_ranges_match_default_theme() -> Result<(), Box<dyn Error>> {
    let docs =
        include_str!("../../../apps/landing/src/app/(detail)/docs/devup/breakpoints/page.mdx");
    let defaults = Theme::default().breakpoints;
    let table = docs
        .split_once("<TableBody>")
        .and_then(|(_, body)| body.split_once("</TableBody>"))
        .map(|(body, _)| body)
        .ok_or_else(|| {
            IoError::new(
                ErrorKind::InvalidData,
                "breakpoints page must contain a ranges table body",
            )
        })?;

    let documented: Vec<(usize, u16, Option<u16>)> = table
        .split("<TableRow>")
        .skip(1)
        .map(|row| -> Result<_, Box<dyn Error>> {
            let cells = row
                .split("<TableCell>")
                .skip(1)
                .map(|cell| {
                    cell.split_once("</TableCell>")
                        .map(|(value, _)| value.trim())
                        .ok_or_else(|| {
                            IoError::new(
                                ErrorKind::InvalidData,
                                "breakpoint table cell must have a closing tag",
                            )
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut cells = cells.into_iter();
            let index = cells
                .next()
                .ok_or_else(|| {
                    IoError::new(
                        ErrorKind::InvalidData,
                        "breakpoint row must contain an index",
                    )
                })?
                .parse()?;
            let range = cells.nth(1).ok_or_else(|| {
                IoError::new(
                    ErrorKind::InvalidData,
                    "breakpoint row must contain a range",
                )
            })?;
            let (start, end) = if let Some(start) = range.strip_suffix("px+") {
                (start, None)
            } else {
                let (start, end) = range.split_once("px - ").ok_or_else(|| {
                    IoError::new(
                        ErrorKind::InvalidData,
                        "breakpoint range must have pixel bounds",
                    )
                })?;
                let end = end.strip_suffix("px").ok_or_else(|| {
                    IoError::new(ErrorKind::InvalidData, "range end must use pixels")
                })?;
                (start, Some(end.parse()?))
            };
            Ok((index, start.parse()?, end))
        })
        .collect::<Result<_, _>>()?;

    let expected: Vec<_> = defaults
        .iter()
        .copied()
        .enumerate()
        .map(|(index, start)| {
            let end = defaults
                .get(index + 1)
                .map(|next| {
                    next.checked_sub(1).ok_or_else(|| {
                        IoError::new(
                            ErrorKind::InvalidData,
                            "default breakpoint range must have a positive upper boundary",
                        )
                    })
                })
                .transpose()?;
            Ok::<_, IoError>((index, start, end))
        })
        .collect::<Result<_, _>>()?;
    assert_eq!(
        documented, expected,
        "public breakpoint range indices and bounds must match Theme::default()"
    );
    Ok(())
}
