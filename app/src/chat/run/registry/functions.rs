//! The `$name(…)` functions, one per name the parser accepts.

use super::language::Function;
use super::{Example, ex};

const fn f(name: &'static str, syntax: &'static str, summary: &'static str, example: Example) -> Function {
    Function {
        name,
        syntax,
        summary,
        example,
    }
}

pub(super) const FUNCTIONS: &[Function] = &[
    f(
        "if",
        "$if(x,then[,else])",
        "then when x isn’t empty, else (or nothing) when it is.",
        ex("$if(2,$2-,trout)", "the second word on, or trout"),
    ),
    f(
        "ifeq",
        "$ifeq(x,text,then[,else])",
        "then when x equals text.",
        ex("$ifeq(1,cake,yummy,meh)", "yummy (for cake)"),
    ),
    f(
        "switch",
        "$switch(x,a:1,b:2,default)",
        "The value after the matching case, or the default.",
        ex("$switch(1,25:50,50:18,25)", "18 (for 50)"),
    ),
    f(
        "join",
        "$join(x,sep)",
        "The words of x joined with sep.",
        ex("$join(1-,/)", "a/b/c (for a b c)"),
    ),
    f(
        "lower",
        "$lower(x)",
        "x in lower case.",
        ex("$lower(Hello)", "hello"),
    ),
    f(
        "upper",
        "$upper(x)",
        "x in upper case.",
        ex("$upper(Hello)", "HELLO"),
    ),
    f(
        "j",
        "$j(path[,default][,sub…])",
        "A value from the JSON a $json call is reading.",
        ex(
            "$json($request(https://example.com/a.json),$j(name))",
            "the name field",
        ),
    ),
    f(
        "replace",
        "$replace(text,search,replacement[,method])",
        "Replaces every search in text, ignoring case. Methods: cs (match case), reg, regFirst, regRef, regFirstRef, regCustom, regFunc (regular expressions).",
        ex("$replace(a-b,-,+)", "a+b"),
    ),
    f(
        "trim",
        "$trim(x)",
        "x without spaces at either end.",
        ex("$trim( hi )", "hi"),
    ),
    f(
        "rand",
        "$rand(a,b,c…)",
        "One of the options, picked at random.",
        ex("$rand(a,b,c)", "b"),
    ),
    f(
        "randnum",
        "$randnum(max) or $randnum(min,max)",
        "A random whole number.",
        ex("$randnum(1,6)", "4"),
    ),
    f(
        "datetime",
        "$datetime(pattern[,zone[,locale[,unix-ms]]])",
        "A date and time in a Java-style pattern or a preset name. Zone is UTC, an offset like +02:00 or empty for local time; the time defaults to now. English only.",
        ex("$datetime(yyyy/MM/dd HH:mm)", "2026/10/15 18:30"),
    ),
    f(
        "urlencode",
        "$urlencode(x)",
        "x ready for a web address.",
        ex("$urlencode(a b)", "a+b"),
    ),
    f(
        "cs",
        "$cs(x)",
        "x with | doubled, for /chain.",
        ex("/chain /echo $cs($1) | /echo done", "the value stays in one part"),
    ),
    f(
        "fs",
        "$fs(x)",
        "x with > doubled, for /foreach.",
        ex("/foreach a b > /echo $fs($1)", "the value stays in one part"),
    ),
    f(
        "sort",
        "$sort(x[,Abc][,sep])",
        "The words of x in order, ignoring case; Abc as the second parameter matches case. Words are split by sep (default a space).",
        ex("$sort(b a c)", "a b c"),
    ),
    f(
        "is",
        "$is(matcher)",
        "true when a highlight-style matcher fits, else empty. The app has no matcher to check yet, so it is always empty.",
        ex("$if($is(x),yes,no)", "no"),
    ),
    f(
        "get",
        "$get(var,key)",
        "A saved value: $get(var,key) after /set var, or $get(username) for the default account’s login.",
        ex("$get(var,lucky)", "7"),
    ),
    f(
        "calc",
        "$calc(expr)",
        "A sum, product, power and so on.",
        ex("$calc(2+3*4)", "14"),
    ),
    f(
        "round",
        "$round(n[,places][,mode][,min])",
        "n rounded.",
        ex("$round(2.567,1)", "2.6"),
    ),
    f(
        "input",
        "$input(message[,initial])",
        "Asks for text. The app doesn’t ask yet, so it cancels the command.",
        ex("/echo $input(Who?)", "(nothing runs)"),
    ),
    f(
        "request",
        "$request(url[,error][,trim][,h[Name: value]])",
        "The body of a web address (GET, 10 seconds, 256 KB at most). http and https only.",
        ex("$request(https://example.com/x.txt,trim)", "the page text"),
    ),
    f(
        "json",
        "$json(text,expr)",
        "Reads text as JSON and evaluates expr with $j() on it.",
        ex("$json({\"a\":1},$j(a))", "1"),
    ),
    f(
        "quote",
        "$quote(x[,quote])",
        "x wrapped in quotes, with any quote inside doubled.",
        ex("$quote(say \"hi\")", "\"say \"\"hi\"\"\""),
    ),
];
