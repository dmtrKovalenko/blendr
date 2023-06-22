use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::{AppRoute, TerminalBackend};

pub struct WelcomeBlock {}

const ASCII_ART: &str = r#"






                      @@@@@@@@@@@
                    @@&         @@@
         @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
        @ @@                             @@@
       @@@ @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
            @@@                         @@@@@@@@/
            @@@         @@    @@@@@@@&  @@      @@@@
            @@@         @@              @@@@@@@@   @@(
            @@@         @@      @@@@@&  @@    @@@  @@@
            @@@         @@              @@    @@@  @@@
            @@@         @@    @@@@@@@&  @@    @@@  @@@
            @@@         @@              @@   @@@   @@
            @@@         @@      @@@@@&  @@@@@@    @@@
            @@@         @@              @@     @@@@
            @@@         @@    *@@@@@@   @@@@@@@@
            @@@                         @@
             @@@                       @@@
              @@@@@@               @@@@@
          @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
          @@@                             @@
          @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
         @@,                               @@@
        @@@            @@@@@@@@@            @@
        @@           @@@      *@@@          @@@
       @@@          @@@         @@           @@
       @@           @@  @@@@@@  @@@          @@@
      @@@           @@@ @@@ @@@ @@
      @@             @@@@ @@  @@@             %@@
     @@@                @@@@@@@                @@
     @@@                                      %@@
       @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
"#;

const SMALL_ASCII_ART: &str = r"





                @@@@@@@@                           
          *****@@*******@@******                    
        @@                      @@                  
        @@@@@@@@@@@@@@@@@@@@@@@@@                   
          @                   @@@@@@@@              
          @       @@          @@      @@            
                  @@     @@@& @@   @@  @@           
          @       @@    ,***  @@   @@  @@           
          @       @@          @@   @@  @#           
          @       @@     @@@& @@@@@   @@            
          @       @@   .@@@@  @@  ,@@@              
          @                   @@@*                  
          @@                  @@                    
         @@@@@@@@@@@@@@@@@@@@@@@@                   
        @@                      @@                  
        @@@@@@@@@@@@@@@@@@@@@@@@@@                  
       @@           ,            @@                 
      @@         @@   /@@        %@                 
      @@       ,@       @@        @@                
     @@        @@ &@@@@ @@         @                
     @@         @@ @@@@/@@         /(               
    @@            @@@@@@           &@               
    ,@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@               
        @@  #@             @@   @@
";

impl AppRoute for WelcomeBlock {
    fn new(_: std::sync::Arc<crate::Ctx>) -> Self {
        Self {}
    }

    fn handle_input(&mut self, _key: &crossterm::event::KeyEvent) {}

    fn render(
        &mut self,
        area: tui::layout::Rect,
        _is_active: bool,
        f: &mut tui::Frame<TerminalBackend>,
    ) -> crate::error::Result<()> {
        if area.height > 25 {
            let paragraph = Paragraph::new(Text::from(if area.height < 30 {
                SMALL_ASCII_ART
            } else {
                ASCII_ART
            }));

            f.render_widget(paragraph, area);
        }

        let block = Block::default().borders(Borders::ALL).title(Span::styled(
            " Welcome to Blendr ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));

        let paragraph = Paragraph::new(vec![Spans::from(
            "We are already scanning for BLE devices. Search for a specific device on the left via arrows or j/k.",
        ), Spans::from(""),  Spans::from(
            "If you know which device you can connect you can use --device <NAME_SEARCH> and --characteristic <CHAR_SEARCH> to connect directly to device.",
        )])
        .wrap(Wrap { trim: true })
        .block(block);

        f.render_widget(paragraph, area);

        Ok(())
    }
}
