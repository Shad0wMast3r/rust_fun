use imap::Session;
use native_tls::TlsConnector;
use std::net::TcpStream;
use dotenv::dotenv;
use std::env;
use chrono::{Duration, Utc};
use std::io::{self, Write};

//Author: Andy Kukuc
//Contributors: CoPilot and Gemini

fn connect_to_yahoo(username: &str, password: &str)
    -> imap::error::Result<Session<native_tls::TlsStream<TcpStream>>> 
{
    let tls = TlsConnector::builder().build().unwrap();
    let tcp_stream = TcpStream::connect(("imap.mail.yahoo.com", 993))?;
    let tls_stream = tls.connect("imap.mail.yahoo.com", tcp_stream)
        .map_err(|e| imap::error::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let mut client = imap::Client::new(tls_stream);
    let session = client.login(username, password).map_err(|e| e.0)?;
    Ok(session)
}

fn list_folders(session: &mut Session<native_tls::TlsStream<TcpStream>>) -> imap::error::Result<Vec<String>> {
    let folders = session.list(None, Some("*"))?;
    let mut names = Vec::new();
    println!("\nAvailable folders/labels:");
    for (i, folder) in folders.iter().enumerate() {
        println!("{}: {}", i + 1, folder.name());
        names.push(folder.name().to_string());
    }
    Ok(names)
}

fn cleanup_folder(session: &mut Session<native_tls::TlsStream<TcpStream>>, folder: &str, days_old: i64) 
    -> imap::error::Result<()> 
{
    session.select(folder)?;
    let cutoff_date = Utc::now().date_naive() - Duration::days(days_old);
    let query = format!("BEFORE {}", cutoff_date.format("%d-%b-%Y"));
    let ids = session.search(query)?;
    if ids.is_empty() {
        println!("No messages older than {} days found in '{}'.", days_old, folder);
        return Ok(());
    }
    let id_list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
    println!("Deleting {} messages from '{}'...", ids.len(), folder);
    session.store(&id_list, "+FLAGS (\\Deleted)")?;
    session.expunge()?;
    println!("Cleanup complete.");
    Ok(())
}

fn main() {
    dotenv().ok();
    let username = env::var("YAHOO_USERNAME").expect("YAHOO_USERNAME not set");
    let password = env::var("YAHOO_APP_PASSWORD").expect("YAHOO_APP_PASSWORD not set");

    match connect_to_yahoo(&username, &password) {
        Ok(mut session) => {
            match list_folders(&mut session) {
                Ok(folders) => {
                    print!("\nEnter the number of the folder to clean: ");
                    io::stdout().flush().unwrap();
                    let mut choice = String::new();
                    io::stdin().read_line(&mut choice).unwrap();
                    if let Ok(index) = choice.trim().parse::<usize>() {
                        if index > 0 && index <= folders.len() {
                            let folder_name = &folders[index - 1];
                            if let Err(e) = cleanup_folder(&mut session, folder_name, 30) {
                                eprintln!("Cleanup failed: {}", e);
                            }
                        } else {
                            println!("Invalid selection.");
                        }
                    }
                }
                Err(e) => eprintln!("Failed to list folders: {}", e),
            }
            session.logout().unwrap();
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

