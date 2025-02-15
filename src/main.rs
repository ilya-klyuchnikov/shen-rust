// [[file:../shen-rust.org::*Preamble][Preamble:1]]

#[macro_use]
extern crate nom;
extern crate uuid;
extern crate time;
extern crate core;
use std::str;
use nom::*;
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use std::rc::Rc;
use std::collections::HashMap;
use std::cell::RefCell;
use uuid::Uuid;
use std::io::{self, Error};
use std::convert::TryFrom;
use std::ops::{Add, Sub, Mul, Div};
use std::fmt;
// Preamble:1 ends here

// [[file:../shen-rust.org::*Token%20Types][Token\ Types:1]]
#[derive(Debug, Clone)]
pub enum KlToken {
    Symbol(String),
    Number(KlNumber),
    String(String),
    Cons(Vec<KlToken>),
    Recur(Vec<KlToken>)
}

#[derive(Debug, Clone)]
pub enum KlNumber {
    Float(f64),
    Int(i64),
}

#[derive(Clone,Debug)]
pub struct UniqueVector {
    uuid: Uuid,
    vector: RefCell<Vec<Rc<KlElement>>>
}

#[derive(Clone,Debug)]
pub enum KlStreamDirection {
    In,
    Out
}

#[derive(Debug)]
pub struct KlFileStream {
    direction : KlStreamDirection,
    file: RefCell<File>
}

#[derive(Clone,Debug)]
pub enum KlStdStream {
    Stdout,
    Stdin
}

#[derive(Debug)]
pub enum KlStream {
    FileStream(KlFileStream),
    Std(KlStdStream)
}

#[derive(Clone,Debug)]
pub enum KlElement {
    Symbol(String),
    Number(KlNumber),
    String(String),
    Cons(Vec<Rc<KlElement>>),
    Closure(KlClosure),
    Vector(Rc<UniqueVector>),
    Stream(Rc<KlStream>),
    Nil,
    Recur(Vec<Rc<KlElement>>)
}

#[derive(Debug,Clone)]
pub enum KlError {
    ErrorString(String)
}

#[derive(Clone)]
pub enum KlClosure {
    FeedMe(Rc<Fn(Rc<KlElement>) -> KlClosure>),
    Thunk(Rc<Fn() -> Rc<KlElement>>),
    Done(Result<Option<Rc<KlElement>>,Rc<KlError>>),
    Trampoline(Rc<Fn() -> Rc<KlElement>>)
}

impl fmt::Debug for KlClosure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &KlClosure::Done(ref s) => write!(f, "{:?}", s.clone()),
            &KlClosure::Thunk(_) => write!(f, "Thunk"),
            &KlClosure::FeedMe(_) => write!(f, "Unsaturated"),
            &KlClosure::Trampoline(_) => write!(f, "Trampoline"),
        }
    }
}
// Token\ Types:1 ends here

// [[file:../shen-rust.org::*Symbol%20Table][Symbol\ Table:1]]
thread_local!(static SYMBOL_TABLE: RefCell<HashMap<String, Rc<KlElement>>> = RefCell::new(HashMap::new()));
// Symbol\ Table:1 ends here

// [[file:../shen-rust.org::*Function%20Table][Function\ Table:1]]
thread_local!(static FUNCTION_TABLE: RefCell<HashMap<String, KlClosure>> = RefCell::new(HashMap::new()));
// Function\ Table:1 ends here

// [[file:../shen-rust.org::*Vector%20Table][Vector\ Table:1]]
thread_local!(static VECTOR_TABLE: RefCell<Vec<(Rc<UniqueVector>, RefCell<Vec<usize>>)>> = RefCell::new(Vec::new()));

pub fn shen_with_unique_vector (unique_vector: &UniqueVector, tx: Box<Fn(&RefCell<Vec<usize>>) -> ()>)
                                -> Option<()> {
    VECTOR_TABLE.with(| vector_table | {
        let vector_table = vector_table.borrow_mut();
        let mut iter = vector_table.iter().take_while(| &tuple | {
            match tuple {
                &(ref vector,_) => {
                    let uuid = vector.uuid;
                    uuid != unique_vector.uuid
                }
            }
        }).peekable();
        let found : Option<&&(Rc<UniqueVector>, RefCell<Vec<usize>>)> = iter.peek();
        match found {
            Some(&&(_, ref indices)) => Some(tx(indices)),
            None => None
        }
    })
}
// Vector\ Table:1 ends here

// [[file:../shen-rust.org::*Symbol%20Character%20Rename%20Table][Symbol\ Character\ Rename\ Table:1]]
thread_local!(static SYMBOL_CHAR_RENAME_TABLE: HashMap<char, &'static str> = {
    let mut table = HashMap::new();
    table.insert('=' ,"__Equal__");
    table.insert('-' ,"__Dash__");
    table.insert('*' ,"__Star__");
    table.insert('/' ,"__Slash__");
    table.insert('+' ,"__Plus__");
    table.insert('?' ,"__Question__");
    table.insert('$' ,"__Dollar__");
    table.insert('!' ,"__Bang__");
    table.insert('@' ,"__At__");
    table.insert('~' ,"__Tilde__");
    table.insert('.' ,"__Dot__");
    table.insert('>' ,"__GT__");
    table.insert('<' ,"__LT__");
    table.insert('&' ,"__And__");
    table.insert('%' ,"__Percent__");
    table.insert('\'',"__Tick__");
    table.insert('#' ,"__Hash__");
    table.insert('`' ,"__BackTick__");
    table.insert(';' ,"__Semi__");
    table.insert(':' ,"__Colon__");
    table.insert('{' ,"__CurlyL__");
    table.insert('}' ,"__CurlyR__");
    table
});

thread_local!(static SYMBOL_CHAR_UNRENAME_TABLE: HashMap<&'static str,char> = {
    let mut table = HashMap::new();
    table.insert("__Equal__"    ,'=');
    table.insert("__Dash__"     ,'-');
    table.insert("__Star__"     ,'*');
    table.insert("__Slash__"    ,'/');
    table.insert("__Plus__"     ,'+');
    table.insert("__Question__" ,'?');
    table.insert("__Dollar__"   ,'$');
    table.insert("__Bang__"     ,'!');
    table.insert("__At__"       ,'@');
    table.insert("__Tilde__"    ,'~');
    table.insert("__Dot__"      ,'.');
    table.insert("__GT__"       ,'>');
    table.insert("__LT__"       ,'<');
    table.insert("__And__"      ,'&');
    table.insert("__Percent__"  ,'%');
    table.insert("__Tick__"     ,'\'');
    table.insert("__Hash__"     ,'#');
    table.insert("__BackTick__" ,'`');
    table.insert("__Semi__"     ,';');
    table.insert("__Colon__"    ,':');
    table.insert("__CurlyL__"   ,'{');
    table.insert("__CurlyR__"   ,'}');
    table
    });
// Symbol\ Character\ Rename\ Table:1 ends here

// [[file:../shen-rust.org::*Symbol%20Keyword%20Rename%20Table][Symbol\ Keyword\ Rename\ Table:1]]
thread_local!(static SYMBOL_KEYWORD_RENAME_TABLE: HashMap<&'static str, &'static str> = {
    let mut table = HashMap::new();
    table.insert("abstract" ,"shen_abstract");
    table.insert("alignof"  ,"shen_alignof");
    table.insert("as"       ,"shen_as");
    table.insert("become"   ,"shen_become");
    table.insert("box"      ,"shen_box");
    table.insert("break"    ,"shen_break");
    table.insert("const"    ,"shen_const");
    table.insert("continue" ,"shen_continue");
    table.insert("crate"    ,"shen_crate");
    table.insert("do"       ,"shen_do");
    table.insert("else"     ,"shen_else");
    table.insert("enum"     ,"shen_enum");
    table.insert("extern"   ,"shen_extern");
    table.insert("false"    ,"shen_false");
    table.insert("final"    ,"shen_final");
    table.insert("fn"       ,"shen_fn");
    table.insert("for"      ,"shen_for");
    table.insert("if"       ,"shen_if");
    table.insert("impl"     ,"shen_impl");
    table.insert("in"       ,"shen_in");
    table.insert("let"      ,"shen_let");
    table.insert("loop"     ,"shen_loop");
    table.insert("macro"    ,"shen_macro");
    table.insert("match"    ,"shen_match");
    table.insert("mod"      ,"shen_mod");
    table.insert("move"     ,"shen_move");
    table.insert("mut"      ,"shen_mut");
    table.insert("offsetof" ,"shen_offsetof");
    table.insert("override" ,"shen_override");
    table.insert("priv"     ,"shen_priv");
    table.insert("proc"     ,"shen_proc");
    table.insert("pub"      ,"shen_pub");
    table.insert("pure"     ,"shen_pure");
    table.insert("ref"      ,"shen_ref");
    table.insert("return"   ,"shen_return");
    table.insert("Self"     ,"shen_Self");
    table.insert("self"     ,"shen_self");
    table.insert("sizeof"   ,"shen_sizeof");
    table.insert("static"   ,"shen_static");
    table.insert("struct"   ,"shen_struct");
    table.insert("super"    ,"shen_super");
    table.insert("trait"    ,"shen_trait");
    table.insert("true"     ,"shen_true");
    table.insert("type"     ,"shen_type");
    table.insert("typeof"   ,"shen_typeof");
    table.insert("unsafe"   ,"shen_unsafe");
    table.insert("unsized"  ,"shen_unsized");
    table.insert("use"      ,"shen_use");
    table.insert("virtual"  ,"shen_virtual");
    table.insert("where"    ,"shen_where");
    table.insert("while"    ,"shen_while");
    table.insert("yield"    ,"shen_yield");
    table
});

thread_local!(static SYMBOL_KEYWORD_UNRENAME_TABLE: HashMap<&'static str, &'static str> = {
    let mut table = HashMap::new();
    table.insert("shen_abstract" ,"abstract");
    table.insert("shen_alignof"  ,"alignof");
    table.insert("shen_as"       ,"as");
    table.insert("shen_become"   ,"become");
    table.insert("shen_box"      ,"box");
    table.insert("shen_break"    ,"break");
    table.insert("shen_const"    ,"const");
    table.insert("shen_continue" ,"continue" );
    table.insert("shen_crate"    ,"crate");
    table.insert("shen_do"       ,"do");
    table.insert("shen_else"     ,"else");
    table.insert("shen_enum"     ,"enum");
    table.insert("shen_extern"   ,"extern");
    table.insert("shen_false"    ,"false");
    table.insert("shen_final"    ,"final");
    table.insert("shen_fn"       ,"fn");
    table.insert("shen_for"      ,"for");
    table.insert("shen_if"       ,"if");
    table.insert("shen_impl"     ,"impl");
    table.insert("shen_in"       ,"in");
    table.insert("shen_let"      ,"let");
    table.insert("shen_loop"     ,"loop");
    table.insert("shen_macro"    ,"macro");
    table.insert("shen_match"    ,"match");
    table.insert("shen_mod"      ,"mod");
    table.insert("shen_move"     ,"move");
    table.insert("shen_mut"      ,"mut");
    table.insert("shen_offsetof" ,"offsetof");
    table.insert("shen_override" ,"override");
    table.insert("shen_priv"     ,"priv");
    table.insert("shen_proc"     ,"proc");
    table.insert("shen_pub"      ,"pub");
    table.insert("shen_pure"     ,"pure");
    table.insert("shen_ref"      ,"ref");
    table.insert("shen_return"   ,"return");
    table.insert("shen_Self"     ,"Self");
    table.insert("shen_self"     ,"self");
    table.insert("shen_sizeof"   ,"sizeof");
    table.insert("shen_static"   ,"static");
    table.insert("shen_struct"   ,"struct");
    table.insert("shen_super"    ,"super");
    table.insert("shen_trait"    ,"trait");
    table.insert("shen_true"     ,"true");
    table.insert("shen_type"     ,"type");
    table.insert("shen_typeof"   ,"typeof");
    table.insert("shen_unsafe"   ,"unsafe");
    table.insert("shen_unsized"  ,"unsized");
    table.insert("shen_use"      ,"use");
    table.insert("shen_virtual"  ,"virtual");
    table.insert("shen_where"    ,"where");
    table.insert("shen_while"    ,"while");
    table.insert("shen_yield"    ,"yield");
    table
});
// Symbol\ Keyword\ Rename\ Table:1 ends here

// [[file:../shen-rust.org::*Helpers][Helpers:1]]
pub fn shen_rename_symbol(symbol : String) -> String {
    SYMBOL_KEYWORD_RENAME_TABLE.with ( | table | {
        match table.get(symbol.as_str()) {
            Some(renamed) => String::from(renamed.clone()),
            None => {
                let mut result = String::new();
                let symbol_characters : Vec<char> = symbol.chars().collect();
                for c in symbol_characters.as_slice() {
                    SYMBOL_CHAR_RENAME_TABLE.with(| table | {
                        match table.get(c) {
                            Some(renamed) => result.push_str(renamed.clone()),
                            _ => result.push(c.clone())
                        }
                    })
                }
                result
            }
        }
    })
}

pub fn shen_unrename_symbol(s : String) -> String {
    SYMBOL_KEYWORD_UNRENAME_TABLE.with(|table|{
        match table.get(s.as_str()) {
            Some(unrenamed) => String::from(unrenamed.clone()),
            None => {
                SYMBOL_CHAR_UNRENAME_TABLE.with(|table| {
                    let mut s = s.clone();
                    let mut keys : Vec<&str> = table.keys().cloned().collect();
                    keys.sort_by(|a,b| b.len().cmp(&a.len()));
                    for k in keys {
                        let new_s = s.clone();
                        let replace_with : char = table.get(k).unwrap().clone();
                        let split : Vec<String> = new_s.as_str().split(k).map(| s | String::from(s)).collect();
                        s = intersperse(split,replace_with.to_string()).clone();
                    }
                    s
                })
            }
        }
    })
}
// Helpers:1 ends here

// [[file:../shen-rust.org::*Constants][Constants:1]]
const CHARACTERS: &'static str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ=-*/+_?$!@~.><&%'#`;:{}";
const DIGITS: &'static str = "0123456789";
// Constants:1 ends here

// [[file:../shen-rust.org::*Parser][Parser:1]]
named!(klsymbol<KlToken>,
       chain!(
       initial: one_of!(CHARACTERS) ~
       remainder: many0!(
           alt_complete!(
               one_of!(DIGITS) |
               one_of!(CHARACTERS)
           )
       ),
       || {
           let mut res : Vec <char> = vec![initial];
           res.extend(remainder);
           KlToken::Symbol(shen_rename_symbol(res.into_iter().collect()))
       })
);
// Parser:1 ends here

// [[file:../shen-rust.org::*Parsers][Parsers:1]]
named!(klnumber<KlToken>,
       alt_complete!(
           chain!(
               n: klfloat,
               || KlToken::Number(n)
           ) |
           chain!(
               n : klint,
               || KlToken::Number(n)
           )
       )
);

named!(klint<KlNumber>,
       chain!(
           sign: opt!(one_of!("-+")) ~
           numbers: many1!(one_of!(DIGITS)),
           || KlNumber::Int(make_int(sign,numbers))
       )
);

named!(klfloat<KlNumber>,
       chain!(
           sign: opt!(one_of!("-+")) ~
           before_decimal: many1!(one_of!(DIGITS)) ~
           one_of!(".") ~
           after_decimal: many1!(one_of!(DIGITS)),
           || KlNumber::Float(make_float(sign,before_decimal, after_decimal))
       )
);
// Parsers:1 ends here

// [[file:../shen-rust.org::*Helpers][Helpers:1]]
fn make_float(sign: Option<char>, before: Vec<char>, after: Vec<char> ) -> f64 {
    let mut float_char_vector : Vec<char> = Vec::new();
    match sign {
        Some(_sign) => float_char_vector.push(_sign),
        None => ()
    };
    float_char_vector.extend(before);
    float_char_vector.push('.');
    float_char_vector.extend(after);
    let float_string : String = float_char_vector.into_iter().collect();
    float_string.parse::<f64>().unwrap()
}

fn make_int(sign: Option<char>, numbers: Vec<char>) -> i64 {
    let mut int_char_vector : Vec<char> = Vec::new();
    match sign {
        Some(_sign) => int_char_vector.push(_sign),
        None => ()
    };
    int_char_vector.extend(numbers);
    let int_string : String = int_char_vector.into_iter().collect();
    let result : i64 = int_string.parse::<i64>().unwrap();
    result
}
// Helpers:1 ends here

// [[file:../shen-rust.org::*Parsers][Parsers:1]]
named!(klstring<KlToken>,
       chain!(
           char!('\"') ~
           contents:  many0!(klstringinnards) ~
           char!('\"'),
           || KlToken::String(make_quoted_string(contents))
       )
);

named!(klstringinnards< &[u8] >,
       escaped!(none_of!("\"\\"), '\\', one_of!("\"n\\"))
);
// Parsers:1 ends here

// [[file:../shen-rust.org::*Helpers][Helpers:1]]
fn make_quoted_string (contents:Vec<&[u8]>) -> String {
    let to_vectors : Vec< Vec<u8> > = contents.iter().map(|c| c.to_vec()).collect();
    let smushed : Vec<u8> = to_vectors.concat();
    let mut quoted : Vec<u8> = Vec::new();
    quoted.push('\"' as u8);
    quoted.extend(smushed);
    quoted.push('\"' as u8);
    let result : String = String::from_utf8(quoted).unwrap();
    result
}
// Helpers:1 ends here

// [[file:../shen-rust.org::*Many%20Until%20Combinator][Many\ Until\ Combinator:1]]
#[macro_export]
macro_rules! many0_until (
    ($input:expr, $stopmac:ident!( $($args:tt)* ), $submac:ident!( $($args2:tt)* )) => (
        {
            let mut res = Vec::new();
            let mut input = $input;
            let mut loop_result = Ok(());

            while input.input_len() != 0 {
                match $stopmac!(input, $($args)*) {
                    IResult::Error(_) => {
                        match $submac!(input, $($args2)*) {
                            IResult::Error(_) => {
                                break;
                            },
                            IResult::Incomplete(Needed::Unknown) => {
                                loop_result = Err(IResult::Incomplete(Needed::Unknown));
                                break;
                            },
                            IResult::Incomplete(Needed::Size(i)) => {
                                let size = i + ($input).input_len() - input.input_len();
                                loop_result = Err(IResult::Incomplete(Needed::Size(size)));
                                break;
                            },
                            IResult::Done(i, o) => {
                                res.push(o);
                                input = i;
                            }
                        }
                    },
                    IResult::Done(_,_) => {
                        break;
                    }
                    IResult::Incomplete(Needed::Unknown) => {
                        loop_result = Err(IResult::Incomplete(Needed::Unknown));
                        break;
                    },
                    IResult::Incomplete(Needed::Size(i)) => {
                        let size = i + ($input).input_len() - input.input_len();
                        loop_result = Err(IResult::Incomplete(Needed::Size(size)));
                        break;
                    },
                }
            }
            match loop_result {
                Ok(()) => IResult::Done(input,res),
                Err(e) => e
            }
        }
    );
    ($i:expr, $stopmac:ident!( $($args:tt)* ), $p:expr) => (
        many0_until!($i, $stopmac!($($args)*), call!($p));
    );
);
// Many\ Until\ Combinator:1 ends here

// [[file:../shen-rust.org::*Parsers][Parsers:1]]
named!(klsexps< Vec<KlToken> >,
       many0!(
           chain!(
               opt!(multispace) ~
               kl: alt_complete!(klsexp|klstring) ~
               opt!(multispace),
               || kl
           )
       )
);

named!(klsexp<KlToken>,
       chain!(
           char!('(') ~
           inner: many0_until!(char!(')'), klsexpinnards) ~
           char!(')'),
           || {
               KlToken::Cons(inner)
           }
       )
);

named!(klsexpinnards<KlToken>,
       chain!(
           opt!(multispace) ~
           atom: alt_complete!(klsexp|klnumber|klstring|klsymbol) ~
           opt!(multispace),
           || atom
       )
);
// Parsers:1 ends here

// [[file:../shen-rust.org::*Collect][Collect:1]]
fn collect_sexps(kl: &[u8], kl_buffer: &mut Vec<Vec<KlToken>>) -> () {
    let mut parsed = match klsexps(kl) {
        IResult::Done(_, out) => out,
        IResult::Incomplete(x) => panic!("incomplete: {:?}", x),
        IResult::Error(e) => panic!("error: {:?}", e),
    };
    // remove toplevel strings
    parsed.retain(|expr| match expr { &KlToken::Cons(_) => true, _ => false });
    // for p in parsed.as_slice() {
    //     println!("{}", intersperse(generate(false, vec![], p), String::from("")));
    // }
    kl_buffer.push(parsed)
}
// Collect:1 ends here

// [[file:../shen-rust.org::*Path%20Utilites][Path\ Utilites:1]]
pub fn add_path (old_path:&Vec<usize>, new_path:Vec<usize>) -> Vec<usize> {
    let mut p = old_path.clone();
    p.extend(new_path);
    p
}
// Path\ Utilites:1 ends here

// [[file:../shen-rust.org::*Getter][Getter:1]]
pub fn get_element_at (path : Vec<usize>, sexp: &KlToken)  -> Option<&KlToken> {
    let mut current_token = sexp;
    for index in path {
        if let &KlToken::Cons(ref current) = current_token {
            if index < current.len() {
                current_token = &current[index];
            }
            else {
                return None;
            }
        }
        else {
            return None;
        }
    }
    Some(current_token)
}
// Getter:1 ends here

// [[file:../shen-rust.org::*Setter][Setter:1]]
pub fn set_element_at(mut path : Vec<usize>, sexp: &mut KlToken, token: KlToken) -> () {
    match (path.pop(), sexp) {
        (Some(p), &mut KlToken::Cons(ref mut vec)) => {
            set_element_at(path, &mut vec[p], token)
        }
        (None, ref mut val) => {
            **val = token;
        }
        _ => panic!("Gah!")
    }
}
// Setter:1 ends here

// [[file:../shen-rust.org::*Mark%20Recur][Mark\ Recur:1]]
pub fn mark_recur(mut path: Vec<usize>, sexp: &mut KlToken) -> () {
    match (path.pop(), sexp) {
        (Some(p), &mut KlToken::Cons(ref mut vec)) => {
            mark_recur(path, &mut vec[p])
        }
        (None, ref mut val) => {
            match val.clone() {
                KlToken::Cons(ref vec) => {
                    let mut new_vec = vec.clone();
                    new_vec.reverse();
                    new_vec.pop();
                    new_vec.reverse();
                    **val = KlToken::Recur(new_vec);
                }
                _ => panic!("Gah!")
            }
        },
        _ => panic!("Gah!")
    }
}
// Mark\ Recur:1 ends here

// [[file:../shen-rust.org::*Detect%20Possible%20Recursive%20Calls][Detect\ Possible\ Recursive\ Calls:1]]
pub fn find_recursive_calls (function_name: String, num_args: usize, sexp: &KlToken) -> Vec<Vec<usize>> {
    let mut found : Vec< Vec<usize> >= Vec::new();
    if let &KlToken::Cons(_) = sexp {
        let mut pending : Vec <(Vec<usize>, &KlToken)> = vec![(Vec::new(), sexp)];
        while pending.len() > 0 {
            let mut newly_found = Vec::new();
            let next = pending.pop().unwrap();
            if let (ref path, &KlToken::Cons(ref current)) = next {
                if let &[KlToken::Symbol(ref s), ref rest @ ..] = current.as_slice() {
                    match (s.as_str(), rest) {
                        (name, rest) if (name == function_name.as_str()) && rest.len() == num_args => {
                            found.push(path.clone());
                        },
                        ("cond", rest) => {
                            let indexed : Vec<(usize, &KlToken)> = rest.iter().enumerate().collect();
                            for (index, sexp) in indexed {
                                if let &KlToken::Cons(ref pair) = sexp {
                                    if let &[_, ref action @ KlToken::Cons(_)] = pair.as_slice() {
                                        newly_found.push((add_path(path, vec![index + 1,1]), action));
                                    }
                                }
                            };
                        },
                        ("if", &[_,ref if_true, ref if_false]) => {
                            if let if_true @ &KlToken::Cons(_) = if_true {
                                newly_found.push((add_path(path, vec![2]), if_true));
                            }
                            if let if_false @ &KlToken::Cons(_) = if_false {
                                newly_found.push((add_path(path, vec![3]), if_false));
                            }
                        },
                        ("trap_error", &[ref to_try, ref handler]) => {
                            if let to_try @ &KlToken::Cons(_) = to_try{
                                newly_found.push((add_path(path, vec![1]), to_try));
                            }
                            if let handler @ &KlToken::Cons(_) = handler {
                                newly_found.push((add_path(path, vec![2]), handler));
                            }
                        },
                        ("let", &[_ , _, ref body @ KlToken::Cons(_)]) |
                        ("defun", &[_ , _, ref body @ KlToken::Cons(_)]) =>
                            newly_found.push((add_path(path, vec![3]), body)),
                        ("lambda", &[_, ref body @ KlToken::Cons(_)]) =>
                            newly_found.push((add_path(path, vec![2]), body)),
                        _ =>
                            match current.last() {
                                Some(ref tail @ &KlToken::Cons(_)) =>
                                    newly_found.push((add_path(path, vec![current.len() - 1]), tail)),
                                _ => ()
                            }
                    }
                }
                else {
                    match current.last() {
                        Some(ref tail @ &KlToken::Cons(_)) =>
                            newly_found.push((add_path(path, vec![current.len() - 1]), tail)),
                        _ => ()
                    }
                }
            }
            newly_found.reverse();
            pending.extend(newly_found);
        }
    }
    found
}
// Detect\ Possible\ Recursive\ Calls:1 ends here

// [[file:../shen-rust.org::*Detect%20Function%20Application%20Context][Detect\ Function\ Application\ Context:1]]
pub fn start_of_function_chain (tail_call_path: Vec<usize>, sexp: &KlToken) -> Option<Vec<usize>> {
    let mut result = None;
    let mut i = 0;
    while i < tail_call_path.len() {
        let current_path : Vec<usize> = tail_call_path.iter().cloned().take(i).collect();
        match get_element_at(current_path.clone(), &sexp) {
            Some(current_element) => {
                if let &KlToken::Cons(ref current) = current_element {
                    match current.as_slice() {
                        &[KlToken::Symbol(ref s), ..] => {
                            match s.as_str() {
                                "if" | "defun" | "let" | "lambda" | "do" => {
                                    result = None;
                                    i = i + 1;
                                }
                                "cond" => {
                                    result = None;
                                    i = i + 2;
                                }
                                _ => {
                                    result = Some(current_path.clone());
                                    i = i + 1
                                }

                            }
                        }
                        _ => ()
                    }
                }
            },
            _ => return None
        }
    }
    result
}
// Detect\ Function\ Application\ Context:1 ends here

// [[file:../shen-rust.org::*Get%20Tail%20Calls][Get\ Tail\ Calls:1]]
pub fn shen_get_all_tail_calls (sexp: &KlToken) -> Vec<Vec<usize>> {
    if let &KlToken::Cons(ref defun) = sexp {
        match defun.as_slice() {
            &[KlToken::Symbol(ref defun), KlToken::Symbol(ref name), KlToken::Cons(ref args), _]
                if defun.as_str() == "defun" => {
                    let mut recursive_calls = find_recursive_calls(name.clone(), args.len(), sexp);
                    recursive_calls.retain(
                        |ref path| {
                            let context = start_of_function_chain(path.iter().cloned().collect(), sexp);
                            match context {
                                Some(_) => false,
                                None => true
                            }
                        }
                    );
                    recursive_calls
                },
            _ => Vec::new()
        }
    }
    else {
        Vec::new()
    }
}
// Get\ Tail\ Calls:1 ends here

// [[file:../shen-rust.org::*Intersperse][Intersperse:1]]
pub fn intersperse(v: Vec<String>, sep: String) -> String {
    if v.len() == 0 {
        String::new()
    }
    else {
        let mut so_far = String::new();
        for i in 0..v.len() {
            so_far = so_far + &(v[i].clone());
            if i != v.len() - 1 {
                so_far = so_far + &sep.clone();
            }
        }
        so_far
    }
}
// Intersperse:1 ends here

// [[file:../shen-rust.org::*Function%20Lookup][Function\ Lookup:1]]
pub fn shen_lookup_function(s: &String) -> Option<KlClosure> {
    FUNCTION_TABLE.with(|table|{
        let table = table.borrow();
        let function = table.get(s);
        match function {
            Some(f) => Some((*f).clone()),
            None => None
        }
    })
}
// Function\ Lookup:1 ends here

// [[file:../shen-rust.org::*Helpers][Helpers:1]]
macro_rules! rc (
    ($input:expr) => ( Rc::new($input) )
);
macro_rules! symbol(
    ($input:expr) => ( KlElement::Symbol(String::from($input)) )
);
macro_rules! error (
    ($input:expr) => ( KlElement::Closure(KlClosure::Done(shen_make_error($input))) )
);

pub fn shen_apply_arguments_to_lambda(l: KlClosure, a: Rc<KlElement>) -> Result<KlClosure, String> {
    match l {
        KlClosure::FeedMe(ref f) => {
            let result = (&f)(a);
            match &result {
                &KlClosure::Done(_) => Ok(result.clone()),
                _ => Err(String::from("Expecting an unsaturated closure."))
            }
        }
        _ => Err(String::from("Expecting an unsaturated closure."))
    }
}

pub fn shen_apply_arguments_to_function(s: String, elements: Vec<Rc<KlElement>>) -> Result<KlClosure, String> {
    match shen_lookup_function(&s) {
        Some(f) => shen_apply_arguments(f.clone(), elements),
        None => Err(format!("Could not find function:{}", s))
    }
}

pub fn shen_apply_arguments(c : KlClosure , elements: Vec<Rc<KlElement>>) -> Result<KlClosure, String> {
    match c {
        KlClosure::FeedMe(_) => {
            let mut so_far : KlClosure = c.clone();
            for e in elements.as_slice() {
                match so_far {
                    KlClosure::FeedMe(f) => so_far = (&f)((*e).clone()),
                    _ => break
                }
            }
            Ok(so_far.clone())
        },
        _ => {
            if elements.len() == 0 {
                Ok(c.clone())
            }
            else {
                Err(String::from("Given a fully saturated closure or thunk"))
            }
        }
    }
}

pub fn shen_apply_element(c: Rc<KlElement>, elements: Vec<Rc<KlElement>>) -> Result<KlClosure, String> {
    match &*c {
        &KlElement::Closure(ref c) => {
            if elements.len() == 0 {
                Ok(c.clone())
            }
            else {
                shen_apply_arguments(c.clone(), elements)
            }
        },
        _ => Err(String::from("Expecting closure."))
    }
}

pub fn shen_closure_to_element(c : KlClosure) -> Rc<KlElement> {
    match c {
        KlClosure::Done(Ok(Some(v))) => v.clone(),
        KlClosure::Done(Ok(None)) => Rc::new(KlElement::Nil),
        _ => Rc::new(KlElement::Closure(c.clone()))
    }
}
// Helpers:1 ends here

// [[file:../shen-rust.org::*Application%20Generation][Application\ Generation:1]]
pub fn generate_apply(is_argument: bool, function_call: String) -> Vec<String> {
    let mut result = Vec::new();
    result.push(format!("match {} {{", function_call));
    if is_argument {
        result.push(String::from("Ok(c) => shen_closure_to_element(c.clone()), \n Err(s) => Rc::new(KlElement::Closure(KlClosure::Done(shen_make_error(s.clone().as_str()))))"));
    }
    else {
        result.push(String::from("Ok(c) => c.clone(), \n Err(s) => KlClosure::Done(shen_make_error(s.clone().as_str()))"))
    }
    result.push(String::from("}"));
    result
}

pub fn shen_apply_function(is_argument: bool, s: String, args: Vec<String>) -> Vec<String> {
    let mut application = Vec::new();
    application.push(format!("shen_apply_arguments_to_function(String::from(\"{}\"), vec![", s));
    application.push(intersperse(args,String::from(",")));
    application.push(String::from("])"));
    generate_apply(is_argument, intersperse(application, String::from("\n")))
}

pub fn shen_apply_arguments_to_curried(is_argument: bool, s: String, args: Vec<String>) -> Vec<String> {
    let mut application = Vec::new();
    application.push(format!("shen_apply_arguments({}, vec![", s));
    application.push(intersperse(args,String::from(",")));
    application.push(String::from("])"));
    generate_apply(is_argument, intersperse(application, String::from("\n")))
}

pub fn shen_apply_argument(is_argument: bool, s: String, args: Vec<String>) -> Vec<String> {
    let mut application = Vec::new();
    application.push(format!("shen_apply_element({}, vec![", s));
    application.push(intersperse(args,String::from(",")));
    application.push(String::from("])"));
    generate_apply(is_argument, intersperse(application, String::from("\n")))
}

pub fn shen_apply_lambda(is_argument: bool, l: String, arg: String) -> Vec<String> {
    let application = format!("shen_apply_arguments_to_lambda({}, {})", l, arg);
    generate_apply(is_argument, application)
}

pub fn clone_bound_variables(bound: Vec<String>) -> String {
    let clone_strings : Vec<String> = bound.iter().map(| v | format!("let {} = {}.clone()", v, v)).collect();
    intersperse(clone_strings, String::from(";")) + ";"
}

pub fn generate_nested_closure(bound: Vec<String>, arg: String ) -> (Vec<String>, String) {
    let mut result : Vec<String> = Vec::new();
    result.push(format!("KlClosure::FeedMe(Rc::new(move |{}| {{", arg));
    result.push(format!("let {}_Copy = (*{}).clone();", arg, arg));
    for b in bound {
        result.push(format!("let {}_Copy = (*{}).clone();", b, b));
        result.push(format!("let {} = {}.clone();", b, b));
    }
    (result, String::from("}))"))
}
// Application\ Generation:1 ends here

// [[file:../shen-rust.org::*Thunk][Thunk:1]]
pub fn generate_thunk(argument: bool, bound: Vec<String>, token: &KlToken) -> Vec<String> {
    let mut result : Vec<String> = Vec::new();
    let mut capture : Vec<String> = Vec::new();
    for b in bound.clone() {
        capture.push(format!("let {}_Copy = {}_Copy.clone();", b, b))
    }
    result.push(format!("Rc::new(KlElement::Closure(KlClosure::Thunk(Rc::new( {{ {} move|| {{ ", intersperse(capture, String::from(""))));
    for b in bound.clone() {
        result.push(format!("let {} = Rc::new({}_Copy.clone());", b, b));
        result.push(format!("let {}_Copy = (*{}).clone();", b ,b))
    }
    result.extend(generate(argument,bound.clone(),token));
    result.push(String::from(" }}))))"));
    result
}
// Thunk:1 ends here

// [[file:../shen-rust.org::*Lambda][Lambda:1]]
pub fn generate_lambda(argument: bool, bound: Vec<String>, token:&KlToken) -> Vec<String> {
    let mut result : Vec<String> = Vec::new();
    if let &KlToken::Cons(ref klif) = &*token {
        match klif.as_slice() {
            &[KlToken::Symbol(ref kllambda), KlToken::Symbol(ref arg) , ref body] if kllambda.as_str() == shen_rename_symbol(String::from("lambda")) => {
                let mut new_bound = bound;
                let (closures, closing) = {
                    new_bound.retain(| x | x != arg);
                    generate_nested_closure(new_bound.clone(), arg.clone())
                };
                new_bound.push(arg.clone());
                if argument {
                    result.push(String::from("Rc::new(KlElement::Closure(\n"));
                }
                result.push(intersperse(closures, String::from("\n")));
                match body {
                    &KlToken::Symbol(ref s) if new_bound.contains(s) =>
                        result.push(format!("KlClosure::Done(Ok(Some({}_Copy.clone())))", s.clone())),
                    _ => result.extend(generate(false, new_bound.clone(), body)),
                }
                result.push(closing);
                if argument {
                    result.push(String::from("))"))
                }
            },
            _ => ()
        }
    }
    result
}
// Lambda:1 ends here

// [[file:../shen-rust.org::*Let][Let:1]]
pub fn generate_let(argument: bool, bound: Vec<String>, token:&KlToken) -> Vec<String> {
    let mut result : Vec<String> = Vec::new();
    if let &KlToken::Cons(ref klif) = &*token {
        match klif.as_slice() {
            &[KlToken::Symbol(ref kllet), ref x @ KlToken::Symbol(_), ref y, ref body] if kllet.as_str() == shen_rename_symbol(String::from("let")) => {
                let lambda_token = KlToken::Cons(vec![KlToken::Symbol(String::from("lambda")), x.clone(), body.clone()]);
                let lambda_string = intersperse(generate_lambda(argument, bound.clone(),&lambda_token), String::from("\n"));
                let args_string = intersperse(generate(true, bound.clone(),y),String::from("\n"));
                result = shen_apply_lambda(argument,lambda_string,args_string);
            },
            _ => ()
        }
    }
    result
}
// Let:1 ends here

// [[file:../shen-rust.org::*Cond][Cond:1]]
pub fn generate_cond(argument:bool, bound: Vec<String>, token:&KlToken) -> Vec<String> {
    let mut result : Vec<String> = Vec::new();
    if let &KlToken::Cons(ref klcond) = &*token {
        match klcond.as_slice() {
            &[KlToken::Symbol(ref klcond), ref cases @ ..] if klcond.as_str() == shen_rename_symbol(String::from("cond")) => {
                let mut pairs = Vec::new();
                let mut pair_list = Vec::new();
                for pair_cons in cases {
                    match pair_cons {
                        &KlToken::Cons(ref pair) => {
                            match pair.as_slice() {
                                &[ref predicate, ref action] => {
                                    let predicate = intersperse(generate_thunk(true,bound.clone(),predicate),String::from("\n"));
                                    let action = intersperse(generate_thunk(true,bound.clone(),action),String::from("\n"));
                                    pairs.push(format!("Rc::new(KlElement::Cons((vec![{},{}])))", action, predicate))
                                },
                                _ => ()
                            }
                        }
                        _ => ()
                    }
                }
                pair_list.push(String::from("Rc::new(KlElement::Cons(vec!["));
                pair_list.push(intersperse(pairs,String::from(",")));
                pair_list.push(String::from("]))"));
                result = shen_apply_function(argument, klcond.clone(), vec![intersperse(pair_list,String::from("\n"))]);
            },
            _ => ()
        }
    }
    result
}
// Cond:1 ends here

// [[file:../shen-rust.org::*Freeze][Freeze:1]]
pub fn generate_freeze(argument: bool, bound: Vec<String>, token:&KlToken) -> Vec<String> {
    let mut result : Vec<String> = Vec::new();
    if let &KlToken::Cons(ref klif) = &*token {
        match klif.as_slice() {
            &[KlToken::Symbol(ref klfreeze), ref a] if klfreeze.as_str() == shen_rename_symbol(String::from("freeze"))=> {
                result = generate_thunk(argument,bound.clone(),a);
            },
            _ => ()
        }
    }
    result
}
// Freeze:1 ends here

// [[file:../shen-rust.org::*And/Or][And/Or:1]]
pub fn generate_and_or(argument: bool, bound: Vec<String>, token:&KlToken) -> Vec<String> {
    let mut result : Vec<String> = Vec::new();
    if let &KlToken::Cons(ref klif) = &*token {
        match klif.as_slice() {
            &[KlToken::Symbol(ref kland_or), ref a, ref b] if kland_or.as_str() == shen_rename_symbol(String::from("and")) || kland_or.as_str() == shen_rename_symbol(String::from("or")) => {
                result = shen_apply_function(argument, kland_or.clone(), vec![
                    intersperse(generate_thunk(true,bound.clone(),a),String::from("\n")),
                    intersperse(generate_thunk(true,bound.clone(),b),String::from("\n"))]);
            },
            _ => ()
        }
    }
    result
}
// And/Or:1 ends here

// [[file:../shen-rust.org::*If][If:1]]
pub fn generate_if(argument: bool, bound: Vec<String>, token: &KlToken) -> Vec<String> {
    let mut result : Vec<String> = Vec::new();
    if let &KlToken::Cons(ref klif) = &*token {
        match klif.as_slice() {
            &[KlToken::Symbol(ref klif), ref predicate, ref if_branch, ref else_branch] if klif.as_str() == shen_rename_symbol(String::from("if")) => {
                result = shen_apply_function(argument, klif.clone(), vec![
                    intersperse(generate(true, bound.clone(),predicate),String::from("\n")),
                    intersperse(generate_thunk(true,bound.clone(),if_branch),String::from("\n")),
                    intersperse(generate_thunk(true,bound.clone(),else_branch),String::from("\n"))
                ]);
            },
            _ => ()
        }
    }
    result
}
// If:1 ends here

// [[file:../shen-rust.org::*Defun][Defun:1]]
pub fn add_to_function_table(name: String, c : KlClosure) {
    FUNCTION_TABLE.with(| function_table | {
        let mut map = function_table.borrow_mut();
        map.insert(shen_rename_symbol(name), c);
    });
}

pub fn splay_out_defun(name: String, args: Vec<KlToken>, body: KlToken) -> KlToken {
    let mut args = args;
    args.reverse();
    args.as_slice().iter().fold(
        body.clone(), | body, arg | {
            KlToken::Cons(vec![KlToken::Symbol(String::from("lambda")),  arg.clone(), body.clone()])
        })
}

pub fn extract_arg_names(args: Vec<KlToken>) -> Vec<String> {
    args.as_slice().iter().filter_map(
        | arg | {
            match arg {
                &KlToken::Symbol(ref s) => Some(s.clone()),
                _ => None
            }
        }
    ).collect()
}

pub fn generate_defun(argument: bool, bound: Vec<String>, token: &KlToken) -> Vec<String> {
    let mut result : Vec<String> = Vec::new();
    if let &KlToken::Cons(ref kldefun) = &*token {
        match kldefun.as_slice() {
            &[KlToken::Symbol(ref kldefun), KlToken::Symbol(ref name), KlToken::Cons(ref args), ref body] if kldefun.as_str() == shen_rename_symbol(String::from("defun")) => {
                let new_body = splay_out_defun(name.clone(), args.clone(), body.clone());
                let mut new_bound = bound.clone();
                new_bound.extend(extract_arg_names(args.clone()));
                result.push(String::from("{"));
                result.push(String::from("let temp = "));
                let arg_names = extract_arg_names(args.clone());
                let mut closures = Vec::new();
                let mut arguments_bound = Vec::new();
                let mut closings = Vec::new();
                for a in arg_names.clone() {
                    let (start, closing) = generate_nested_closure(arguments_bound.clone(), a.clone());
                    arguments_bound.push(a);
                    closures.extend(start);
                    closings.push(closing);
                }
                result.extend(closures.clone());
                let inner = generate(argument, new_bound.clone(), body);
                match &*body {
                    &KlToken::Cons(_) => {
                        let paths = shen_get_all_tail_calls(token);
                        let mut token = token.clone();
                        for p in paths.clone() {
                            let mut p = p;
                            p.reverse();
                            mark_recur(p.clone(), &mut token);
                        }
                        // println!("{:?}", token);
                        if paths.len() > 0 {
                            if let &KlToken::Cons(ref marked_defun) = &token {
                                if let &[_,_,_,ref body] = marked_defun.as_slice() {
                                    let inner = generate(true, new_bound.clone(), body);
                                    let mut trampoline = Vec::new();
                                    trampoline.push(String::from("{"));
                                    trampoline.push(String::from("let trampoline = | "));
                                    trampoline.push(intersperse(arg_names.clone().iter().map(| a | format!("{} : Rc<KlElement>", a)).collect(), String::from(",")));
                                    trampoline.push(String::from("| {"));
                                    for a in arg_names.clone() {
                                        trampoline.push(format!("let {}_Copy : KlElement = (*{}).clone();", a, a))
                                    }
                                    trampoline.extend(inner.clone());
                                    trampoline.push(String::from("};"));
                                    trampoline.push(format!("let mut done= None;"));
                                    trampoline.push(
                                        format!(
                                            "let mut current_args = vec![{}];",
                                            intersperse(
                                                arg_names.clone().iter().map(| a | format!("{}.clone()", a)).collect(),
                                                String::from(","))
                                        )
                                    );
                                    trampoline.push(format!("while !done.is_some() {{"));
                                    trampoline.push(
                                        format!("let result = trampoline({});",
                                                intersperse(arg_names.clone().iter().enumerate().map(| (i,_) | {
                                                    format!("current_args[{}].clone()", i)
                                                }).collect(),
                                                            String::from(",")))
                                    );
                                    trampoline.push(
                                        format!("match &*result {{ &KlElement::Recur(ref v) => current_args = v.clone(), output => done = Some(KlClosure::Done(Ok(Some(result.clone())))) }};")
                                    );
                                    trampoline.push(String::from("}"));
                                    trampoline.push(String::from("done.unwrap()"));
                                    trampoline.push(String::from("}"));
                                    result.push(intersperse(trampoline.clone(), String::from("\n")));
                                }
                            }
                        }
                        else {
                            result.extend(inner.clone());
                        }
                    }
                    _ => {
                        result.push(String::from("KlClosure::Done(Ok(Some("));
                        result.extend(inner.clone());
                        result.push(String::from(")))"));
                    }
                }
                result.push(intersperse(closings.clone(), String::from("\n")));
                result.push(String::from(";"));
                result.push(format!("add_to_function_table(String::from(\"{}\"), temp.clone())", name.clone()));
                result.push(String::from("}"));
            },
            _ => ()
        }
    }
    result
}
// Defun:1 ends here

// [[file:../shen-rust.org::*Atoms][Atoms:1]]
pub fn generate_atoms(argument: bool, bound: Vec<String>, token: &KlToken) -> Vec<String> {
    match token {
        &KlToken::Number(KlNumber::Int(i)) => vec![format!("Rc::new(KlElement::Number(KlNumber::Int({})))", i)],
        &KlToken::Number(KlNumber::Float(i)) => vec![format!("Rc::new(KlElement::Number(KlNumber::Float({})))", i)],
        &KlToken::String(ref s) => vec![format!("Rc::new(KlElement::String(String::from({})))", s.clone())],
        &KlToken::Symbol(ref s) => {
            if bound.contains(s) {
                vec![format!("Rc::new({}_Copy.clone())", s.clone())]
            }
            else {
                vec![format!("Rc::new(KlElement::Symbol(String::from(\"{}\")))", s.clone())]
            }
        },
        _ => Vec::new()
    }
}
// Atoms:1 ends here

// [[file:../shen-rust.org::*Application][Application:1]]
pub fn generate_application(argument: bool, bound: Vec<String>, token: &KlToken) -> Vec<String> {
    let mut result = Vec::new();
    match &*token {
        &KlToken::Cons(ref application) => {
            match application.as_slice() {
                &[ref app @ KlToken::Cons(_), ref rest @ ..] => {
                    let args = rest.into_iter().map(| e | intersperse(generate(true, bound.clone(), e),String::from("\n"))).collect();
                    result = shen_apply_arguments_to_curried(argument, intersperse(generate(false, bound.clone(),app),String::from("\n")), args);
                },
                &[KlToken::Symbol(ref s), ref rest @ ..] => {
                    let args = rest.into_iter().map(| e | intersperse(generate(true, bound.clone(), e),String::from("\n"))).collect();
                    if bound.contains(s) {
                        result = shen_apply_argument(
                            argument,
                            format!("Rc::new({}_Copy.clone())", s.clone()),
                            args);
                    }
                    else {
                        result = shen_apply_function(argument, s.clone(), args);
                    }
                },
                &[] => result = vec![String::from("Rc::new(KlElement::Cons(vec![]))")],
                _ => panic!("Trying to apply something other than a symbol or cons.")
            }
        },
        &KlToken::Recur(ref args) => {
            // println!("{:?}", args);
            let arg_tuple : Vec<String> = args.into_iter().map(| e | intersperse(generate(true, bound.clone(), e),String::from("\n"))).collect();
            let mut args = Vec::new();
            args.push(String::from("Rc::new(KlElement::Recur(vec!["));
            args.push(intersperse(arg_tuple, String::from(",")));
            args.push(String::from("]))"));
            result = args;
        },
        _ => panic!("Not a cons list or recurrence.")
    }
    result
}
// Application:1 ends here

// [[file:../shen-rust.org::*Generate][Generate:1]]
pub fn generate(argument: bool, bound: Vec<String>, token: &KlToken) -> Vec<String> {
    let mut result : Vec<String> = Vec::new();
    let generators : Vec<Box<Fn(bool, Vec<String>, &KlToken) -> Vec<String>>>
        = vec![
            Box::new(generate_atoms),
            Box::new(generate_defun),
            Box::new(generate_cond),
            Box::new(generate_if),
            Box::new(generate_and_or),
            Box::new(generate_lambda),
            Box::new(generate_let),
            Box::new(generate_freeze),
            Box::new(generate_application)
        ];
    for g in generators.as_slice() {
        if result.len() == 0 {
            result = g(argument, bound.clone(),token);
        }
        else {
            break;
        }
    }
    result
}
// Generate:1 ends here

// [[file:../shen-rust.org::*Helpers][Helpers:1]]
pub fn shen_symbol_to_string(s : &KlElement) -> Result<Rc<&String>, Rc<String>> {
    match s {
        &KlElement::Symbol(ref s) => Ok(Rc::new(&s)),
        _ => Err(Rc::new(String::from("shen_symbol_to_string: Expecting a symbol.")))
    }
}

pub fn shen_string_to_symbol(s : &str) -> Rc<KlElement> {
    Rc::new(KlElement::Symbol(String::from(s)))
}

pub fn shen_is_bool (a: Rc<KlElement>) -> bool {
    match &*a {
        &KlElement::Symbol(ref s) if s.as_str() == "shen_true" || s.as_str() == "shen_false" => true,
        _ => false
    }
}

pub fn shen_is_thunk(a: Rc<KlElement>) -> bool {
    match &*a {
        &KlElement::Closure(KlClosure::Thunk(_)) => true,
        _ => false
    }
}

pub fn shen_force_thunk(a : Rc<KlElement>) -> Result<Option<Rc<KlElement>>,Rc<KlError>> {
    match &*a {
        &KlElement::Closure(KlClosure::Thunk(ref inner)) => Ok(Some(inner())),
        _ => shen_make_error("shen_force_thunk: Expected a thunk.")
    }
}

pub fn shen_make_error(s : &str) -> Result<Option<Rc<KlElement>>, Rc<KlError>> {
    Err(Rc::new((KlError::ErrorString(String::from(s)))))
}

pub fn shen_atoms_equal(a: Rc<KlElement>, b: Rc<KlElement>) -> Result<bool, (Vec<Rc<KlElement>>, Vec<Rc<KlElement>>)> {
    match (&*a, &*b) {
        (&KlElement::Symbol(ref i), &KlElement::Symbol(ref j)) if (*i).as_str() == (*j).as_str() => Ok(true),
        (&KlElement::Number(KlNumber::Int(i)), &KlElement::Number(KlNumber::Int(j))) if i == j => Ok(true),
        (&KlElement::Number(KlNumber::Float(i)), &KlElement::Number(KlNumber::Float(j))) if i == j => Ok(true),
        (&KlElement::String(ref i), &KlElement::String(ref j)) if (*i).as_str() == (*j).as_str() => Ok(true),
        (&KlElement::Cons(ref i), &KlElement::Cons(ref j)) => Err(((*i).clone(),(*j).clone())),
        (&KlElement::Vector(ref i), &KlElement::Vector(ref j)) =>
            match (&**i,&**j) {
                (&UniqueVector{uuid: _, vector: ref i}, &UniqueVector{ uuid: _, vector: ref j}) =>
                    Err((i.borrow().clone(),j.borrow().clone()))
            },
        _ => Ok(false)
    }
}

pub fn shen_vector_equal(a: &Vec<Rc<KlElement>>, b: &Vec<Rc<KlElement>>) -> bool {
    let mut inner_vectors : Vec<(Rc<KlElement>, Rc<KlElement>)>=
        (*a).clone().into_iter().zip((*b).clone().into_iter()).collect();
    let mut still_equal = (*a).len() == (*b).len();
    let mut next = inner_vectors.pop();
    while still_equal && next.is_some() {
        let (a,b) = next.unwrap();
        match shen_atoms_equal(a,b) {
            Ok(equal_or_not) => {
                still_equal = equal_or_not;
            },
            Err((i,j))=> {
                let new_inner_vector : Vec<(Rc<KlElement>, Rc<KlElement>)> =
                    i.clone().into_iter().zip(j.clone().into_iter()).collect();
                inner_vectors.extend(new_inner_vector.clone());
                still_equal = (*i).len() == (*j).len();
            }
        }
        next = inner_vectors.pop();
    }
    still_equal
}
// Helpers:1 ends here

// [[file:../shen-rust.org::*If][If:1]]
pub fn shen_if () -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | predicate | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | if_thunk | {
                            let predicate = predicate.clone();
                            KlClosure::FeedMe(
                                Rc::new(
                                    move | else_thunk | {
                                        if !shen_is_bool(predicate.clone()) {
                                            KlClosure::Done(shen_make_error("shen_if: the predicate must be 'true' or 'false'."))
                                        }
                                        else {
                                            if !shen_is_thunk(if_thunk.clone()) || !shen_is_thunk(else_thunk.clone()) {
                                                KlClosure::Done(shen_make_error("shen_if: Both the if and else branch must be thunks."))
                                            }
                                            else {
                                                match *predicate {
                                                    KlElement::Symbol(ref s) if s.as_str() == "shen_true" => {
                                                        KlClosure::Done(shen_force_thunk(if_thunk.clone()))
                                                    },
                                                    KlElement::Symbol(ref s) if s.as_str() == "shen_false" => {
                                                        KlClosure::Done(shen_force_thunk(else_thunk.clone()))
                                                    },
                                                    _ => KlClosure::Done(shen_make_error("Expecting predicate to be 'true' or 'false'."))
                                                }
                                            }
                                        }
                                    }
                                )
                            )
                        }
                    )
                )
            }
        )
    )
}
// If:1 ends here

// [[file:../shen-rust.org::*And][And:1]]
pub fn shen_and () -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | a_thunk | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | b_thunk | {
                            if !shen_is_thunk(a_thunk.clone()) || !shen_is_thunk(b_thunk.clone()) {
                                KlClosure::Done(shen_make_error("shen_and: Both arguments must be thunks."))
                            }
                            else {
                                let forced = shen_force_thunk(a_thunk.clone()).unwrap();
                                if forced.is_some() && !shen_is_bool(forced.clone().unwrap()) {
                                    KlClosure::Done(shen_make_error("shen_and: The first argument must evaluate to the symbol 'true' or 'false."))
                                }
                                else {
                                    let forced : Rc<KlElement> = forced.unwrap();
                                    match &*forced {
                                        &KlElement::Symbol(ref a)
                                            if a.as_str() == "shen_false" =>
                                            KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_false")))),
                                        _ => {
                                            let forced = shen_force_thunk(b_thunk).unwrap();
                                            if forced.is_some() && !shen_is_bool(forced.clone().unwrap()) {
                                                KlClosure::Done(shen_make_error("shen_and: The second argument must evaluate to the symbol 'true' or 'false."))
                                            }
                                            else {
                                                let forced = forced.unwrap();
                                                match &*forced {
                                                    &KlElement::Symbol(ref b)
                                                        if b.as_str() == "shen_false" =>
                                                        KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_false")))),
                                                    _ => KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_true"))))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    )
                )
            }
        )
    )
}
// And:1 ends here

// [[file:../shen-rust.org::*Or][Or:1]]
pub fn shen_or () -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | a_thunk | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | b_thunk | {
                            if !shen_is_thunk(a_thunk.clone()) || !shen_is_thunk(b_thunk.clone()) {
                                KlClosure::Done(shen_make_error("shen_or: Both arguments must be thunks."))
                            }
                            else {
                                let forced = shen_force_thunk(a_thunk.clone()).unwrap();
                                if forced.is_some() && !shen_is_bool(forced.clone().unwrap()) {
                                    KlClosure::Done(shen_make_error("shen_or: The first argument must evaluate to the symbol 'true' or 'false."))
                                }
                                else {
                                    let forced : Rc<KlElement> = forced.unwrap();
                                    match &*forced {
                                        &KlElement::Symbol(ref a)
                                            if a.as_str() == "shen_true" =>
                                            KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_true")))),
                                        _ => {
                                            let forced = shen_force_thunk(b_thunk).unwrap();
                                            if forced.is_some() && !shen_is_bool(forced.clone().unwrap()) {
                                                KlClosure::Done(shen_make_error("shen_or: The second argument must evaluate to the symbol 'true' or 'false."))
                                            }
                                            else {
                                                let forced = forced.unwrap();
                                                match &*forced {
                                                    &KlElement::Symbol(ref b)
                                                        if b.as_str() == "shen_true" =>
                                                        KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_true")))),
                                                    _ => KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_false"))))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    )
                )
            }
        )
    )
}
// Or:1 ends here

// [[file:../shen-rust.org::*Cond][Cond:1]]
pub fn shen_cond() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | cases | {
                match &*cases {
                    &KlElement::Cons(ref case_pairs) => {
                        let mut pairs : Vec<(Rc<KlElement>,Rc<KlElement>)>= Vec::new();
                        for case in case_pairs {
                            match &**case {
                                &KlElement::Cons(ref pair) if pair.len() == 2 => {
                                    let ref predicate = pair[1];
                                    let ref action = pair[0];
                                    if !shen_is_thunk(predicate.clone()) || !shen_is_thunk(action.clone()) {
                                        return KlClosure::Done(shen_make_error("shen_cond: All cases must be a pairs of thunks."))
                                    }
                                    else {
                                        pairs.push((predicate.clone(),action.clone()))
                                    }
                                },
                                _ => return KlClosure::Done(shen_make_error("shen_cond: All cases must be pairs."))
                            }
                        };
                        let mut result = None;
                        for &(ref predicate,ref action) in pairs.as_slice() {
                            let forced = shen_force_thunk(predicate.clone()).unwrap();
                            if forced.is_some() && !shen_is_bool(forced.clone().unwrap()) {
                                result = Some(KlClosure::Done(shen_make_error("shen_cond: All predicates must evaluate to 'true' or 'false'.")))
                            }
                            else {
                                let forced = forced.unwrap();
                                match &*forced {
                                    &KlElement::Symbol(ref s) if s.as_str() == "shen_true" => {
                                        let forced = shen_force_thunk(action.clone()).unwrap();
                                        result = Some(KlClosure::Done(Ok(forced)));
                                        break;
                                    },
                                    _ => ()
                                }
                            }
                        }
                        match result {
                            Some(r) => r,
                            None => KlClosure::Done(shen_make_error("shen_cond: None of the predicates evaluated to 'true'."))

                        }
                    },
                    _ => KlClosure::Done(shen_make_error("shen_cond: All cases must be a pairs of thunks."))
                }
            }
        )
    )
}
// Cond:1 ends here

// [[file:../shen-rust.org::*Intern][Intern:1]]
pub fn shen_intern() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | string | {
                match &*string {
                    &KlElement::String(ref s) => {
                        KlClosure::Done(Ok(Some(Rc::new(KlElement::Symbol(s.clone())))))
                    },
                    _ => KlClosure::Done(shen_make_error("shen_intern: expecting a string."))
                }
            }
        )
    )
}
// Intern:1 ends here

// [[file:../shen-rust.org::*pos][pos:1]]
pub fn shen_pos() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | string | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | number | {
                            let string = string.clone();
                            match &*string {
                                &KlElement::String(ref s) => {
                                    let length = (&s).chars().count();
                                    match &*number {
                                        &KlElement::Number(KlNumber::Int(i)) if i > 0 && (i as usize) < length => {
                                            let char = (*s).chars().nth(i as usize).unwrap();
                                            let mut result = String::from("");
                                            result.push(char);
                                            KlClosure::Done(Ok(Some(Rc::new(KlElement::String(result)))))
                                        },
                                        _ => KlClosure::Done(shen_make_error("shen_pos: expecting a number between 0 and the length of the string."))
                                    }
                                },
                                _ => KlClosure::Done(shen_make_error("shen_pos: expecting a string."))
                            }
                        }
                    )
                )
            }
        )
    )
}
// pos:1 ends here

// [[file:../shen-rust.org::*tlstr][tlstr:1]]
pub fn shen_tlstr() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | string | {
                match &*string {
                    &KlElement::String(ref s) => {
                        let length = (&s).chars().count();
                        if length == 0 {
                            KlClosure::Done(shen_make_error("shen_tlstr: expecting non-empty string."))
                        }
                        else {
                            let (_, tail) = (&s).split_at(1);
                            KlClosure::Done(Ok(Some(Rc::new(KlElement::String(String::from(tail))))))
                        }
                    },
                    _ => KlClosure::Done(shen_make_error("shen_pos: expecting a string."))
                }

            }
        )
    )
}
// tlstr:1 ends here

// [[file:../shen-rust.org::*cn][cn:1]]
pub fn shen_cn () -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | string_a | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | string_b | {
                            let string_a = string_a.clone();
                            match (&*string_a, &*string_b) {
                                (&KlElement::String(ref a), &KlElement::String(ref b)) => {
                                    KlClosure::Done(Ok(Some(Rc::new(KlElement::String((*a).clone() + b)))))
                                },
                                _ => KlClosure::Done(shen_make_error("shen_cn: expecting two strings."))
                            }

                        }
                    )
                )
            }
        )
    )
}
// cn:1 ends here

// [[file:../shen-rust.org::*str][str:1]]
pub fn shen_str() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | atom | {
                match &*atom {
                    &KlElement::String(_) => KlClosure::Done(Ok(Some(atom.clone()))),
                    &KlElement::Number(KlNumber::Int(i)) =>
                        KlClosure::Done(Ok(Some(Rc::new(KlElement::String(format!("{}", i)))))),
                    &KlElement::Number(KlNumber::Float(f)) =>
                        KlClosure::Done(Ok(Some(Rc::new(KlElement::String(format!("{}", f)))))),
                    &KlElement::Symbol(ref s) =>
                        KlClosure::Done(Ok(Some(Rc::new(KlElement::String(shen_unrename_symbol(s.clone())))))),
                    &KlElement::Stream(ref s) => {
                        match &**s {
                            &KlStream::FileStream(_) =>
                                KlClosure::Done(Ok(Some(Rc::new(KlElement::String(String::from("<file stream>")))))),
                            &KlStream::Std(KlStdStream::Stdout) =>
                                KlClosure::Done(Ok(Some(Rc::new(KlElement::String(String::from("<stdout>")))))),
                            &KlStream::Std(KlStdStream::Stdin) =>
                                KlClosure::Done(Ok(Some(Rc::new(KlElement::String(String::from("<stdin>")))))),
                        }
                    }
                    _ => KlClosure::Done(shen_make_error("Not an atom, stream or closure; str cannot convert it to a string."))
                }
            }
        )
    )
}
// str:1 ends here

// [[file:../shen-rust.org::*string?][string\?:1]]
pub fn shen_stringp() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | element | {
                match &*element {
                    &KlElement::String(_) =>
                        KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_true")))),
                    _ => KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_false"))))
                }
            }
        )
    )
}
// string\?:1 ends here

// [[file:../shen-rust.org::*n->string][n->string:1]]
pub fn shen_n_to_string() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | n | {
                match &*n {
                    &KlElement::Number(KlNumber::Int(i)) => {
                        let convert : Result<u8, _>= TryFrom::try_from(i);
                        match convert {
                            Ok(char) => {
                                match String::from_utf8(vec![char]) {
                                    Ok(string) => {
                                        KlClosure::Done(Ok(Some(Rc::new(KlElement::String(string)))))
                                    },
                                    Err(_) =>
                                        KlClosure::Done(shen_make_error("shen_n_to_string: number is not utf8."))
                                }
                            },
                            Err(_) => KlClosure::Done(shen_make_error("shen_n_to_string: number could not be converted to u8."))
                        }
                    },
                    _ => KlClosure::Done(shen_make_error("shen_n_to_string: expecting an integer."))
                }
                }
        )
    )
}
// n->string:1 ends here

// [[file:../shen-rust.org::*string->n][string->n:1]]
pub fn shen_string_to_n() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | string | {
                match &*string {
                    &KlElement::String(ref s) if s.len() == 1 => {
                        let v : Vec<u8> = (*s.clone()).into();
                        KlClosure::Done(Ok(Some(Rc::new(KlElement::Number(KlNumber::Int(v[0] as i64))))))
                    },
                    _ => KlClosure::Done(shen_make_error("shen_string_to_n: expecting a unit string."))

                }
            }
        )
    )
}
// string->n:1 ends here

// [[file:../shen-rust.org::*simple-error][simple-error:1]]
pub fn shen_simple_error () -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | error | {
                match *error {
                    KlElement::String(ref s) => {
                        KlClosure::Done(shen_make_error(&s.as_str()))
                    },
                    _ => KlClosure::Done(shen_make_error("shen_simple_error: Expecting a string."))
                }
            }
        )
    )
}
// simple-error:1 ends here

// [[file:../shen-rust.org::*trap-error][trap-error:1]]
pub fn shen_trap_error() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | to_try_thunk | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | handler | {
                            let to_try_thunk = to_try_thunk.clone();
                            if !shen_is_thunk(to_try_thunk.clone()) {
                                KlClosure::Done(shen_make_error("shen_trap_error: Expecting a thunk."))
                            }
                            else {
                                match &*handler {
                                    &KlElement::Closure(KlClosure::FeedMe(ref f)) => {
                                        let forced = shen_force_thunk(to_try_thunk.clone());
                                        match forced {
                                            Ok(r) => { KlClosure::Done(Ok(r)) },
                                            Err(s) => match &*s {
                                                &KlError::ErrorString(ref s) => {
                                                    let exception = Rc::new(KlElement::String(s.clone()));
                                                    (&f)(exception.clone())
                                                }
                                            }
                                        }
                                    },
                                    _ => KlClosure::Done(shen_make_error("Expecting a closure."))
                                }
                            }
                        }
                    )
                )
            }
        )
    )
}
// trap-error:1 ends here

// [[file:../shen-rust.org::*error-to-string][error-to-string:1]]
pub fn shen_error_to_string() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | exception | {
                match &*exception {
                    &KlElement::String(ref s) => {
                        KlClosure::Done(Ok(Some(Rc::new(KlElement::String(s.clone())))))
                    },
                    _ => KlClosure::Done(shen_make_error("shen_error_to_string: expecting a string."))
                }
            }
        )
    )
}
// error-to-string:1 ends here

// [[file:../shen-rust.org::*Set][Set:1]]
pub fn shen_set () -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | symbol | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | value | {
                            let symbol = symbol.clone();
                            SYMBOL_TABLE.with(| symbol_table | {
                                let mut map = symbol_table.borrow_mut();
                                let symbol_string = shen_symbol_to_string(&*symbol);
                                match symbol_string {
                                    Ok(s) => {
                                        map.insert((*s).clone(), value);
                                        return KlClosure::Done(Ok(None))
                                    }
                                    _ => return KlClosure::Done(shen_make_error("shen_set: expecting a symbol for a key."))
                                }
                            })
                        }
                    )
                )
            }
        )
    )
}
// Set:1 ends here

// [[file:../shen-rust.org::*Value][Value:1]]
pub fn shen_value() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | symbol | {
                SYMBOL_TABLE.with(| symbol_table| {
                    let map = symbol_table.borrow();
                    let symbol_string = shen_symbol_to_string(&*symbol);
                    match symbol_string {
                        Ok(s) => {
                            match map.get(*s) {
                                Some(v) => KlClosure::Done(Ok(Some(v.clone()))),
                                None => KlClosure::Done(shen_make_error(&*(format!("variable {} is unbound", (*s)))))
                            }
                        },
                        _ => return KlClosure::Done(shen_make_error("shen_value: expecting a symbol for a key."))
                    }
                })
            }
        )
    )
}
// Value:1 ends here

// [[file:../shen-rust.org::*Cons][Cons:1]]
pub fn shen_cons() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | new_head | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | list | {
                            let new_head = new_head.clone();
                            match *list {
                                KlElement::Cons(ref cons_cells) => {
                                    let mut new_cons_cells = cons_cells.clone();
                                    new_cons_cells.push(new_head.clone());
                                    KlClosure::Done(Ok(Some(Rc::new(KlElement::Cons(new_cons_cells)))))
                                },
                                _ => KlClosure::Done(shen_make_error("shen_cons: Expecting a list."))
                            }
                        }
                    )
                )
            }
        )
    )
}
// Cons:1 ends here

// [[file:../shen-rust.org::*Head][Head:1]]
pub fn shen_hd() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | list | {
                match *list {
                    KlElement::Cons(ref cons_cells) => {
                        let head = cons_cells.last();
                        match head {
                            Some(hd) => KlClosure::Done(Ok(Some(hd.clone()))),
                            None => KlClosure::Done(Ok(None))
                        }
                    },
                    _ => KlClosure::Done(shen_make_error("shen_hd: Expecting a list"))

                }
            }
        )
    )
}
// Head:1 ends here

// [[file:../shen-rust.org::*Tail][Tail:1]]
pub fn shen_tl() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | list | {
                match *list {
                    KlElement::Cons(ref cons_cells) => {
                        let mut new_cons_cells = cons_cells.clone();
                        let popped = new_cons_cells.pop();
                        match popped {
                            Some(_) => KlClosure::Done(Ok(Some(Rc::new(KlElement::Cons(new_cons_cells))))),
                            _ => KlClosure::Done(Ok(Some(Rc::new(KlElement::Cons(vec![])))))
                        }
                    },
                    _ => KlClosure::Done(shen_make_error("shen_tl: Expecting a list."))
                }
            }
        )
    )
}
// Tail:1 ends here

// [[file:../shen-rust.org::*Cons?][Cons\?:1]]
pub fn shen_consp() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | list | {
                match *list {
                    KlElement::Cons(_) => KlClosure::Done(Ok(Some(Rc::new(KlElement::Symbol(String::from("shen_true")))))),
                    _ => KlClosure::Done(Ok(Some(Rc::new(KlElement::Symbol(String::from("shen_false"))))))
                }
            }
        )
    )
}
// Cons\?:1 ends here

// [[file:../shen-rust.org::*=][=:1]]
pub fn shen_equal() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | a | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | b | {
                            let a = a.clone();
                            let is_equal =
                                match shen_atoms_equal(a,b) {
                                    Ok(equal) => equal,
                                    Err((ref v1, ref v2)) => shen_vector_equal(v1,v2)
                                };
                            KlClosure::Done(
                                Ok(Some((shen_string_to_symbol(
                                    if is_equal {"shen_true"} else {"shen_false"}))))
                            )
                        }
                    )
                )
            }
        )
    )
}
// =:1 ends here

// [[file:../shen-rust.org::*absvector][absvector:1]]
pub fn shen_absvector() -> KlClosure {
    let v = Vec::new();
    let uuid = Uuid::new_v4();
    let unique_vector = Rc::new(UniqueVector{ uuid: uuid, vector: RefCell::new(v) });
    VECTOR_TABLE.with(| vector_map | {
        let mut vector_map = vector_map.borrow_mut();
        vector_map.push((unique_vector.clone(), RefCell::new(Vec::new())));
    });
    KlClosure::Done(Ok(Some(Rc::new(KlElement::Vector(unique_vector)))))
}
// absvector:1 ends here

// [[file:../shen-rust.org::*address->][address->:1]]
pub fn shen_insert_at_address() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | vector | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | index | {
                            let vector = vector.clone();
                            KlClosure::FeedMe(
                                Rc::new(
                                    move | value | {
                                        match &*vector {
                                            &KlElement::Vector(ref unique_vector) => {
                                                match *index {
                                                    KlElement::Number(KlNumber::Int(i)) if i >= 0 => {
                                                        let mut payload = (**unique_vector).vector.borrow_mut();
                                                        let length = payload.len();
                                                        if i as usize <= length {
                                                            payload[i as usize] = value.clone();
                                                            match &*value {
                                                                &KlElement::Vector(_) | &KlElement::Cons(_) => {
                                                                    let tx = Box::new(
                                                                        move | ref_cell : &RefCell<Vec<usize>> | {
                                                                            let mut v = (*ref_cell).borrow_mut();
                                                                            v.push(i.clone() as usize);
                                                                        }
                                                                    );
                                                                    shen_with_unique_vector(&unique_vector, tx);
                                                                },
                                                                _ => ()
                                                            };
                                                            KlClosure::Done(Ok(Some(vector.clone())))
                                                        }
                                                        else {
                                                            KlClosure::Done(shen_make_error("shen_insert_at_address: Expecting a positive integer less than the vector length."))
                                                        }
                                                    },
                                                    _ => KlClosure::Done(shen_make_error("shen_insert_at_address: Expecting a positive number."))
                                                }
                                            },
                                            _ => KlClosure::Done(shen_make_error("shen_insert_at_address: Expecting a vector."))
                                        }
                                    }
                                )
                            )
                        }
                    )
                )
            }
        )
    )
}
// address->:1 ends here

// [[file:../shen-rust.org::*<-address][<-address:1]]
pub fn shen_get_at_address() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | vector | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | index | {
                            let vector = vector.clone();
                            match &*vector {
                                &KlElement::Vector(ref unique_vector) => {
                                    match *index {
                                        KlElement::Number(KlNumber::Int(i)) if i > 0 => {
                                            let payload = (**unique_vector).vector.borrow();
                                            let length = payload.len();
                                            if i as usize <= length {
                                                let ref found = payload[i as usize];
                                                KlClosure::Done(Ok(Some((*found).clone())))
                                            }
                                            else {
                                                KlClosure::Done(Ok(None))
                                            }
                                        },
                                        _ => KlClosure::Done(shen_make_error("shen_insert_at_address: Expecting a positive number."))
                                    }
                                },
                                _ => KlClosure::Done(shen_make_error("shen_insert_at_address: Expecting a vector."))
                            }
                        }
                    )
                )
            }
        )
    )
}
// <-address:1 ends here

// [[file:../shen-rust.org::*absvector?][absvector\?:1]]
pub fn shen_absvectorp() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | vector | {
                match &*vector {
                    &KlElement::Vector(_) => KlClosure::Done(Ok(Some(Rc::new(KlElement::Symbol(String::from("shen_true")))))),
                    _ => KlClosure::Done(Ok(Some(Rc::new(KlElement::Symbol(String::from("shen_false")))))),
                }
            }
        )
    )
}
// absvector\?:1 ends here

// [[file:../shen-rust.org::*write-byte][write-byte:1]]
pub fn shen_write_byte () -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | to_write | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | stream | {
                            let byte = to_write.clone();
                            match &*byte {
                                &KlElement::Number(KlNumber::Int(i)) => {
                                    let converted = TryFrom::try_from(i);
                                    match converted {
                                        Ok(byte) => {
                                            match *stream {
                                                KlElement::Stream(ref stream) => {
                                                    let stream : &KlStream = &*stream;
                                                    match stream {
                                                        &KlStream::FileStream(KlFileStream { direction: KlStreamDirection::Out, file: ref handle }) => {
                                                            let mut file = (*handle).borrow_mut();
                                                            let written = file.write(&[byte]);
                                                            match written {
                                                                Ok(_) => KlClosure::Done(Ok(Some(to_write.clone()))),
                                                                Err(_) => KlClosure::Done(shen_make_error("shen_write_byte: Could not write byte to file."))
                                                            }
                                                        },
                                                        &KlStream::Std(KlStdStream::Stdout) => {
                                                            let written = io::stdout().write(&[byte]);
                                                            match written {
                                                                Ok(_) => KlClosure::Done(Ok(Some(to_write.clone()))),
                                                                Err(_) => KlClosure::Done(shen_make_error("shen_write_byte: Could not write byte to stdout."))
                                                            }
                                                        }
                                                        _ => KlClosure::Done(shen_make_error("shen_write_byte: Expecting a write-only stream or stdout."))
                                                    }
                                                },
                                                _ => KlClosure::Done(shen_make_error("shen_write_byte: Expecting a stream."))
                                            }
                                        },
                                        Err(_) => KlClosure::Done(shen_make_error("shen_write_byte: Expecting a byte."))
                                    }
                                },
                                _ => KlClosure::Done(shen_make_error("shen_write_byte: Expecting a number."))
                            }
                        }
                    )
                )
            }
        )
    )
}
// write-byte:1 ends here

// [[file:../shen-rust.org::*read-byte][read-byte:1]]
pub fn shen_read_byte () -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            move | stream | {
                match *stream {
                    KlElement::Stream(ref stream) => {
                        let stream : &KlStream = &*stream;
                        let mut buffer = [0; 1];
                        let read = match stream {
                            &KlStream::FileStream(KlFileStream { direction: KlStreamDirection::In, file: ref handle }) => {
                                let mut file = (*handle).borrow_mut();
                                let mut buffer = [0;1];
                                file.read(&mut buffer[..])
                            },
                            &KlStream::Std(KlStdStream::Stdin) => {
                                io::stdin().read(&mut buffer[..])
                            }
                            _ => Err(Error::new(std::io::ErrorKind::Other, "shen_write_byte: Expecting a write-only stream or stdout."))
                        };
                        match read {
                            Ok(_) => {
                                let read : Result<i64,_> = TryFrom::try_from(buffer[0]);
                                match read {
                                    Ok(i) => KlClosure::Done(Ok(Some(Rc::new(KlElement::Number(KlNumber::Int(i)))))),
                                    Err(_) => KlClosure::Done(shen_make_error("shen_read_byte: Could not read a byte."))
                                }
                            },
                            Err(_) => KlClosure::Done(shen_make_error("shen_write_byte: Could not read byte."))
                        }

                    },
                    _ => KlClosure::Done(shen_make_error("shen_write_byte: Expecting a stream."))
                }
            }
        )
    )
}
// read-byte:1 ends here

// [[file:../shen-rust.org::*Open][Open:1]]
pub fn shen_open() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | file_name | {
                KlClosure::FeedMe(
                    Rc::new(
                        move | direction | {
                            let file_name = file_name.clone();
                            match &*file_name {
                                &KlElement::String(ref path) => {
                                    let path = path.as_str();
                                    match &*direction {
                                        &KlElement::Symbol(ref direction) if direction.as_str() == "in" => {
                                            match File::open(path) {
                                                Ok(f) =>
                                                    KlClosure::Done(
                                                        Ok(Some(Rc::new(KlElement::Stream(Rc::new(
                                                            KlStream::FileStream(
                                                                KlFileStream {
                                                                    direction: KlStreamDirection::In,
                                                                    file: RefCell::new(f)}))))))),
                                                _ => KlClosure::Done(shen_make_error("shen_open: Could not open file."))
                                            }
                                        },
                                        _ => KlClosure::Done(shen_make_error("shen_open: Expecting direction 'in'."))
                                    }
                                },
                                _ => KlClosure::Done(shen_make_error("shen_open: Expecting a file path."))
                            }
                        }
                    )
                )
            }
        )
    )
}
// Open:1 ends here

// [[file:../shen-rust.org::*get-time][get-time:1]]
pub fn shen_get_time() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | time_type | {
                match &*time_type {
                    &KlElement::Symbol(ref s) if s.as_str() == "run" || s.as_str() == "real" => {
                        KlClosure::Done(Ok(Some(Rc::new(KlElement::Number(KlNumber::Float(time::precise_time_s()))))))
                    }
                    _ => KlClosure::Done(shen_make_error("shen_open: Expecting 'run' or 'real'."))
                }
            }
        )
    )
}
// get-time:1 ends here

// [[file:../shen-rust.org::*Macros][Macros:1]]
macro_rules! number_op {
    ($a:ident, $b:ident, $checked_op:ident, $float_op:ident, $fn_name:expr, $op_name:expr) => {
        KlClosure::FeedMe(
            Rc::new(
                | $a | {
                    KlClosure::FeedMe(
                        Rc::new(
                            move | $b | {
                                let $a = $a.clone();
                                match (&*$a, &*$b) {
                                    (&KlElement::Number(KlNumber::Int(a)), &KlElement::Number(KlNumber::Int(b))) => {
                                        match a.$checked_op(b) {
                                            Some(i) => KlClosure::Done(Ok(Some(Rc::new(KlElement::Number(KlNumber::Int(i.clone())))))),
                                            _ =>
                                                KlClosure::Done(shen_make_error(format!("{}: {} would cause overflow.", $fn_name, $op_name).as_str()))
                                        }
                                    },
                                    (&KlElement::Number(KlNumber::Float(a)), &KlElement::Number(KlNumber::Int(b))) => {
                                        KlClosure::Done(Ok(Some(Rc::new(KlElement::Number(KlNumber::Float(a.$float_op(b as f64)))))))
                                    }
                                    (&KlElement::Number(KlNumber::Int(a)), &KlElement::Number(KlNumber::Float(b))) => {
                                        KlClosure::Done(Ok(Some(Rc::new(KlElement::Number(KlNumber::Float((a as f64).$float_op(b)))))))
                                    }
                                    (&KlElement::Number(KlNumber::Float(a)), &KlElement::Number(KlNumber::Float(b))) => {
                                        KlClosure::Done(Ok(Some(Rc::new(KlElement::Number(KlNumber::Float(a.$float_op(b)))))))
                                    }
                                    _ => KlClosure::Done(shen_make_error(format!("{}: expecting two numbers.", $fn_name).as_str()))
                                }
                            }
                        )
                    )
                }
            )
        )
    }
}

macro_rules! number_test {
    ($a:ident, $b:ident, $test:ident, $fn_name:expr) => {
        KlClosure::FeedMe(
            Rc::new(
                | $a | {
                    KlClosure::FeedMe(
                        Rc::new(
                            move | $b | {
                                let $a = $a.clone();
                                let test_result =
                                    match (&*$a, &*$b) {
                                        (&KlElement::Number(KlNumber::Int(a)), &KlElement::Number(KlNumber::Int(b))) => Some($test(a,&b)),
                                        (&KlElement::Number(KlNumber::Float(a)), &KlElement::Number(KlNumber::Int(b))) => Some($test(a,&(b as f64))),
                                        (&KlElement::Number(KlNumber::Int(a)), &KlElement::Number(KlNumber::Float(b))) => Some($test((a as f64), &b)),
                                        (&KlElement::Number(KlNumber::Float(a)), &KlElement::Number(KlNumber::Float(b))) => Some($test(a,&b)),
                                        _ => None
                                    };
                                match test_result {
                                    Some(true) => KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_true")))),
                                    Some(false) => KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_false")))),
                                    None => KlClosure::Done(shen_make_error(format!("{}: expecting two numbers.", $fn_name).as_str()))
                                }
                            }
                        )
                    )
                }
            )
        )
    }
}
// Macros:1 ends here

// [[file:../shen-rust.org::*Helpers][Helpers:1]]
pub fn shen_le_shim<T: PartialEq + PartialOrd>(a: T, b: &T) -> bool {
    a.le(&b)
}
pub fn shen_ge_shim<T: PartialEq + PartialOrd>(a: T, b: &T) -> bool {
    a.ge(&b)
}
pub fn shen_eq_ge_shim<T: PartialEq + PartialOrd>(a: T, b: &T) -> bool {
    a.ge(&b) || a.eq(&b)
}
pub fn shen_eq_le_shim<T: PartialEq + PartialOrd>(a: T, b: &T) -> bool {
    a.le(&b) || a.eq(&b)
}
// Helpers:1 ends here

// [[file:../shen-rust.org::*+][+:1]]
pub fn shen_plus() -> KlClosure {
    number_op!(number_a, number_b, checked_add, add, "shen_plus", "adding")
}
// +:1 ends here

// [[file:../shen-rust.org::**][*:1]]
pub fn shen_mul() -> KlClosure {
    number_op!(number_a, number_b, checked_mul, mul, "shen_mul", "multiplying")
}
// *:1 ends here

// [[file:../shen-rust.org::*-][-:1]]
pub fn shen_sub() -> KlClosure {
    number_op!(number_a, number_b, checked_sub, sub, "shen_sub", "subtracting")
}
// -:1 ends here

// [[file:../shen-rust.org::*/][/:1]]
pub fn shen_div() -> KlClosure {
    number_op!(number_a, number_b, checked_div, div, "shen_div", "dividing")
}
// /:1 ends here

// [[file:../shen-rust.org::*>][>:1]]
pub fn shen_ge() -> KlClosure {
    number_test!(number_a, number_b, shen_ge_shim, "shen_ge")
}
// >:1 ends here

// [[file:../shen-rust.org::*<][<:1]]
pub fn shen_le() -> KlClosure {
    number_test!(number_a, number_b, shen_le_shim, "shen_le")
}
// <:1 ends here

// [[file:../shen-rust.org::*>=][>=:1]]
pub fn shen_eq_le() -> KlClosure {
    number_test!(number_a, number_b, shen_eq_le_shim, "shen_le")
}
// >=:1 ends here

// [[file:../shen-rust.org::*<=][<=:1]]
pub fn shen_eq_ge() -> KlClosure {
    number_test!(number_a, number_b, shen_eq_ge_shim, "shen_le")
}
// <=:1 ends here

// [[file:../shen-rust.org::*number?][number\?:1]]
pub fn shen_numberp() -> KlClosure {
    KlClosure::FeedMe(
        Rc::new(
            | number | {
                match &*number {
                    &KlElement::Number(_) => KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_true")))),
                    _ => KlClosure::Done(Ok(Some(shen_string_to_symbol("shen_false"))))
                }
            }
        )
    )
}
// number\?:1 ends here

// [[file:../shen-rust.org::*Filling%20The%20Function%20Table][Filling\ The\ Function\ Table:1]]
pub fn shen_fill_function_table() {
    FUNCTION_TABLE.with(| function_table | {
        let mut map = function_table.borrow_mut();
        map.insert(shen_rename_symbol(String::from("shen_if"))         ,shen_if());
        map.insert(shen_rename_symbol(String::from("and"))             ,shen_and());
        map.insert(shen_rename_symbol(String::from("or"))              ,shen_or());
        map.insert(shen_rename_symbol(String::from("cond"))            ,shen_cond());
        map.insert(shen_rename_symbol(String::from("intern"))          ,shen_intern());
        map.insert(shen_rename_symbol(String::from("pos"))             ,shen_pos());
        map.insert(shen_rename_symbol(String::from("tlstr"))           ,shen_tlstr());
        map.insert(shen_rename_symbol(String::from("cn"))              ,shen_cn());
        map.insert(shen_rename_symbol(String::from("str"))             ,shen_str());
        map.insert(shen_rename_symbol(String::from("string?"))         ,shen_stringp());
        map.insert(shen_rename_symbol(String::from("n->string"))       ,shen_n_to_string());
        map.insert(shen_rename_symbol(String::from("string->n"))       ,shen_string_to_n());
        map.insert(shen_rename_symbol(String::from("simple-error"))    ,shen_simple_error());
        map.insert(shen_rename_symbol(String::from("trap-error"))      ,shen_trap_error());
        map.insert(shen_rename_symbol(String::from("error-to-string")) ,shen_error_to_string());
        map.insert(shen_rename_symbol(String::from("set"))             ,shen_set());
        map.insert(shen_rename_symbol(String::from("value"))           ,shen_value());
        map.insert(shen_rename_symbol(String::from("cons"))            ,shen_cons());
        map.insert(shen_rename_symbol(String::from("hd"))              ,shen_hd());
        map.insert(shen_rename_symbol(String::from("tl"))              ,shen_tl());
        map.insert(shen_rename_symbol(String::from("cons?"))           ,shen_consp());
        map.insert(shen_rename_symbol(String::from("="))               ,shen_equal());
        map.insert(shen_rename_symbol(String::from("absvector"))       ,shen_absvector());
        map.insert(shen_rename_symbol(String::from("address->"))       ,shen_insert_at_address());
        map.insert(shen_rename_symbol(String::from("<-address"))       ,shen_get_at_address());
        map.insert(shen_rename_symbol(String::from("absvector?"))      ,shen_absvectorp());
        map.insert(shen_rename_symbol(String::from("write-byte"))      ,shen_write_byte());
        map.insert(shen_rename_symbol(String::from("read-byte"))       ,shen_read_byte());
        map.insert(shen_rename_symbol(String::from("open"))            ,shen_open());
        map.insert(shen_rename_symbol(String::from("get-time"))        ,shen_get_time());
        map.insert(shen_rename_symbol(String::from("+"))               ,shen_plus());
        map.insert(shen_rename_symbol(String::from("*"))               ,shen_mul());
        map.insert(shen_rename_symbol(String::from("-"))               ,shen_sub());
        map.insert(shen_rename_symbol(String::from("/"))               ,shen_div());
        map.insert(shen_rename_symbol(String::from(">"))               ,shen_ge());
        map.insert(shen_rename_symbol(String::from("<"))               ,shen_le());
        map.insert(shen_rename_symbol(String::from("<="))              ,shen_eq_le());
        map.insert(shen_rename_symbol(String::from(">="))              ,shen_eq_ge());
        map.insert(shen_rename_symbol(String::from("number?"))         ,shen_numberp());
    })
}
// Filling\ The\ Function\ Table:1 ends here

// [[file:../shen-rust.org::*KLambda%20Files][KLambda\ Files:1]]

const KLAMBDAFILES: &'static [ &'static str ] = &[
    "toplevel.kl", "core.kl", "sys.kl", "sequent.kl", "yacc.kl",
    "reader.kl", "prolog.kl", "track.kl", "load.kl", "writer.kl",
    "macros.kl", "declarations.kl", "types.kl", "t-star.kl"
];
// KLambda\ Files:1 ends here

// [[file:../shen-rust.org::*KLambda%20Files][KLambda\ Files:2]]
fn main () {
    shen_fill_function_table();
    let with_klambda_path : Vec<String> = KLAMBDAFILES
        .into_iter()
        .map(|f| {"KLambda/".to_string() + f})
        .collect();
    for f in with_klambda_path {
        let path = Path::new(&f);
        let mut kl : Vec<Vec<KlToken>>= Vec::new();
        match File::open(path) {
            Ok(mut f) => {
                let mut buffer : Vec<u8> = Vec::new();
                match f.read_to_end(&mut buffer) {
                    Ok(_) => {
                        collect_sexps(&buffer, &mut kl);
                        println!("{:?}", kl);
                    },
                    Err(e) => panic!("error: {:?}", e)
                }
            },
            Err(e) => panic!("error: {:?}", e)
        }
    }
}
// KLambda\ Files:2 ends here
