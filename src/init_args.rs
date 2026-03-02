use syn::{Ident, LitStr};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

use rust_macro::*;

#[inline]
fn parse_first<T: Parse>(input: ParseStream) -> Result<T> {
    input.parse()
}

#[inline]
fn parse_next<T: Parse>(input: ParseStream) -> Result<T> {
    input.parse::<Token![,]>()?;
    input.parse()
}

#[derive(Accessors)]
pub struct InitArgs {
    path: LitStr,
}

impl Parse for InitArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let path = parse_first(input)?;
        Ok(Self { path })
    }
}

#[derive(Accessors)]
pub struct InitArgs2 {
    path: LitStr,
    tag: Ident,
}

impl Parse for InitArgs2 {
    fn parse(input: ParseStream) -> Result<Self> {
        let path = parse_first(input)?;
        let tag = parse_next(input)?;
        Ok(Self { path, tag })
    }
}

#[derive(Accessors)]
pub struct InitArgs2LitStr {
    path: LitStr,
    tag: LitStr,
}

impl Parse for InitArgs2LitStr {
    fn parse(input: ParseStream) -> Result<Self> {
        let path = parse_first(input)?;
        let tag = parse_next(input)?;
        Ok(Self { path, tag })
    }
}

#[derive(Accessors)]
pub struct InitArgs3_2LitStr {
    path: LitStr,
    tag: LitStr,
    tag2: Ident,
}

impl Parse for InitArgs3_2LitStr {
    fn parse(input: ParseStream) -> Result<Self> {
        let path = parse_first(input)?;
        let tag2 = parse_next(input)?;
        let tag = parse_next(input)?;
        Ok(Self { path, tag, tag2 })
    }
}

#[derive(Accessors)]
pub struct InitArgs3 {
    path: LitStr,
    tag: Ident,
    tag2: Ident,
}

impl Parse for InitArgs3 {
    fn parse(input: ParseStream) -> Result<Self> {
        let path = parse_first(input)?;
        let tag = parse_next(input)?;
        let tag2 = parse_next(input)?;
        Ok(Self { path, tag, tag2 })
    }
}

#[derive(Accessors)]
pub struct InitArgs4 {
    path: LitStr,
    tag: Ident,
    tag2: Ident,
    tag3: Ident,
}

impl Parse for InitArgs4 {
    fn parse(input: ParseStream) -> Result<Self> {
        let path = parse_first(input)?;
        let tag = parse_next(input)?;
        let tag2 = parse_next(input)?;
        let tag3 = parse_next(input)?;
        Ok(Self { path, tag, tag2, tag3 })
    }
}