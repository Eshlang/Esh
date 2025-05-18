use codegen::codegen::CodeGen;
use codegen::Compiler;
use codegen::Parser;
use detemplater::Detemplater;
use dfbin::DFBin;
use templater::Templater;
use core::str;
use std::env;
use std::fs;
use std::io::BufRead;
use std::rc::Rc;
use websocket::ClientBuilder;
use websocket::Message;
use std::net::TcpStream;
use websocket::sync::Client;
use std::{thread, time};
use lexer::Lexer;

use clap::{Arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;

fn main() {
    let matches = Command::new("esh")
        .about("A CLI tool for manipulating .dfa, .dfbin, and .esh files")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("compile")
                .about("Compiles an esh file into .dfa and .dfbin")
                .arg(Arg::new("input")
                    .help("Path to input .esh file")
                    .required(true)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("dfa_out")
                    .help("Optional output .dfa file path")
                    .required(false)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("dfbin_out")
                    .help("Optional output .dfbin file path")
                    .required(false)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("place")
                    .short('c')
                    .help("Place the templates using CodeClient API.")
                    .action(ArgAction::SetTrue))
        )
        .subcommand(
            Command::new("assemble")
                .about("Compiles a .dfa file into templates")
                .arg(Arg::new("input")
                    .help("Path to input .dfa file")
                    .required(true)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("output")
                    .help("Optional output .dfbin path")
                    .required(false)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("place")
                    .short('c')
                    .help("Place the templates using CodeClient API.")
                    .action(ArgAction::SetTrue))
        )
        .subcommand(
            Command::new("template")
                .about("Templatizes a .dfbin file into code templates")
                .arg(Arg::new("input")
                    .help("Path to input .dfbin file")
                    .required(true)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("output")
                    .help("Optional output .txt path")
                    .required(false)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("place")
                    .short('c')
                    .help("Place the templates using CodeClient API.")
                    .action(ArgAction::SetTrue))
        )
        .subcommand(
            Command::new("disassemble")
                .about("Disassembles a .dfbin file into a .dfa file.")
                .arg(Arg::new("input")
                    .help("Path to input .esh file")
                    .required(true)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("dfbin_in")
                    .help(".dfbin file path input")
                    .required(true)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("dfa_out")
                    .help(".dfa file path output")
                    .required(false)
                    .value_parser(clap::value_parser!(PathBuf)))
        )
        .subcommand(
            Command::new("detemplate")
                .about("Detemplatizes a bunch of templates into .dfbin")
                .arg(Arg::new("input")
                    .help("Path to input .esh file")
                    .required(true)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("templates_in")
                    .help("Templates input file, .txt with gzip separated by newlines")
                    .required(true)
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("dfbin_out")
                    .help(".dfbin file path output")
                    .required(true)
                    .value_parser(clap::value_parser!(PathBuf)))
        )
        .get_matches();

    match matches.subcommand() {
        Some(("compile", sub_m)) => handle_compile(sub_m),
        Some(("assemble", sub_m)) => handle_assemble(sub_m),
        Some(("template", sub_m)) => handle_template(sub_m),
        Some(("disassemble", sub_m)) => handle_disassemble(sub_m),
        Some(("detemplate", sub_m)) => handle_detemplate(sub_m),
        _ => unreachable!("Clap should ensure a valid subcommand"),
    }
}

fn handle_compile(matches: &ArgMatches) {
    let input = matches.get_one::<PathBuf>("input").unwrap();
    let dfa_out = matches.get_one::<PathBuf>("dfa_out");
    let dfbin_out = matches.get_one::<PathBuf>("dfbin_out");
    let place = matches.get_flag("place");


    let file_bytes = fs::read(input).expect("File should read");
    let lexer = Lexer::new(str::from_utf8(&file_bytes).expect("Should encode to utf-8"));
    let lexer_tokens: Vec<Rc<lexer::types::Token>> = lexer.map(|v| Rc::new(v.expect("Lexer token should unwrap"))).collect();
    
    println!("LEXER TOKENS\n----------------------\n{:#?}\n----------------------", lexer_tokens);
    let mut parser = Parser::new(lexer_tokens.as_slice());
    let parser_tree = Rc::new(parser.parse().expect("Parser statement block should unwrap"));
    //##println!("PARSER TREE\n----------------------\n{:#?}\n----------------------", parser_tree);
    
    let mut codegen = CodeGen::new();
    codegen.codegen_from_node(parser_tree.clone()).expect("Codegen should generate");


    let code = codegen.buffer.flush();
    if let Some(dfbin_path) = dfbin_out {
        code.write_to_file(&dfbin_path.clone().into_os_string().into_string().expect("Should unwrap OS string for .dfbin output")).expect("DFBin should write");
    }
    if let Some(dfa_path) = dfa_out {
        let mut decompiler = decompiler::Decompiler::new(code.clone()).expect("Decompiler should create");
        decompiler.set_capitalization(decompiler::decompiler::DecompilerCapitalization::lowercase);
        let decompiled = decompiler.decompile().expect("Decompiler should decompile");
        //##println!("DECOMPILED\n----------------------\n{}\n----------------------", decompiled);
        fs::write(dfa_path, decompiled).expect("Decompiled DFA should write.");
    }
    if place {
        codeclient_send_bin(code.clone());
    }

}

fn handle_assemble(matches: &ArgMatches) {
    let input = matches.get_one::<PathBuf>("input").unwrap();
    let output = matches.get_one::<PathBuf>("output");
    let place = matches.get_flag("place");
    let file_data = fs::read(input).expect(".dfa file path should be valid.");
    let mut compiler = Compiler::new(str::from_utf8(file_data.as_slice()).unwrap());
    let compiled_dfbin = compiler.compile_string().expect("Compiling should be valid.").clone();
    if let Some(dfbin_path) = output {
        compiled_dfbin.clone().write_to_file(&dfbin_path.clone().into_os_string().into_string().expect("Should unwrap OS string for .dfbin output")).expect("Compiled file should save");
    }
    if place {
        codeclient_send_bin(compiled_dfbin.clone());
    }
}

fn handle_template(matches: &ArgMatches) {
    let input = matches.get_one::<PathBuf>("input").unwrap();
    let output = matches.get_one::<PathBuf>("output");
    let place = matches.get_flag("place");
    let compiled_dfbin = dfbin::DFBin::from_file(&input.clone().into_os_string().into_string().expect("Should unwrap OS string for .dfbin input")).expect("File should read into dfbin.");
    let mut templater = Templater::from_bin(compiled_dfbin.clone());
    if let Ok(templates) = templater.to_templates() {
        if let Some(output_path) = output {
            let _ = fs::write(output_path, templates.join("\n"));
        }
    };  
    
    if place {
        codeclient_send_bin(compiled_dfbin.clone());
    }
}

fn handle_disassemble(matches: &ArgMatches) {
    let input = matches.get_one::<PathBuf>("input").unwrap();
    let output = matches.get_one::<PathBuf>("output").unwrap();
    let compiled_dfbin = dfbin::DFBin::from_file(&input.clone().into_os_string().into_string().expect("Should unwrap OS string for .dfbin input")).expect("File should read into dfbin.");
    let mut decompiler = decompiler::Decompiler::new(compiled_dfbin.clone()).expect("Decompiler should create");
    decompiler.set_capitalization(decompiler::decompiler::DecompilerCapitalization::lowercase);
    let decompiled = decompiler.decompile().expect("Decompiler should decompile");
    //##println!("DECOMPILED\n----------------------\n{}\n----------------------", decompiled);
    fs::write(output, decompiled).expect("Decompiled DFA should write.");
    
}

fn handle_detemplate(matches: &ArgMatches) {
    let input = matches.get_one::<PathBuf>("input").unwrap();
    let output = matches.get_one::<PathBuf>("output").unwrap();
    
    let mut detemplater = Detemplater::new();
    let binding = fs::read(input).expect("Input file should be read.").to_owned();
    let input_lines = binding.lines();
    for template in input_lines {
        let Ok(template) = template else { break; };
        let _ = detemplater.append_template_str(&template);
    };
    
    let bin = detemplater.result();
    bin.clone().write_to_file(&output.clone().into_os_string().into_string().expect("Should unwrap OS string for .dfbin output")).expect("Compiled file should save");
}


fn codeclient_connect() -> Result<Client<TcpStream>, i32> {
    use websocket::OwnedMessage::Text;
    let mut client = ClientBuilder::new("ws://localhost:31375")
    .unwrap()
    .connect_insecure()
    .unwrap();

    client.send_message(&Message::text("scopes default inventory movement read_plot write_code")).unwrap();

    let Text(response) = client.recv_message().unwrap() else {
        return Err(0);
    };
    if response != "auth" {
        return Err(1);
    }
    return Ok(client);
}

fn codeclient_send_bin(bin: DFBin) {
    let mut templater = Templater::from_bin(bin);
    match templater.to_templates() {
        Ok(templates) => {
            println!("Successfully processed templates! Templates:\n\n{}", templates.join("\n\n"));
            let mut client = codeclient_connect().unwrap();
            codeclient_send_templates(&mut client, templates).unwrap();
        }
        Err(e) => {
            println!("Templater error while templating: {}", e);
            assert!(false)
        }
    };
}

fn codeclient_send_templates(client: &mut Client<TcpStream>, templates: Vec<String>) -> Result<(), i32> {
    println!("Sending templates...");
    let time_wait = time::Duration::from_millis(100);
    client.send_message(&Message::text("place")).unwrap();
    thread::sleep(time_wait);
    for template in templates {
        let template_message = format!("place {}", template);
        client.send_message(&Message::text(template_message)).unwrap();
        println!("\rSent template {}...", template);
        thread::sleep(time_wait);
    }
    client.send_message(&Message::text("place go")).unwrap();
    thread::sleep(time_wait);

    return Ok(());
}