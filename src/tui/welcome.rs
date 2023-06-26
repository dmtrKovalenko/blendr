use tui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
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

        let block = Block::default().borders(Borders::ALL).border_type(tui::widgets::BorderType::Rounded).title(Span::styled(
            " Welcome to Blendr ",
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        ));


        let paragraph = Paragraph::new(
        vec![
            Line::from(""),
            Line::from(
                "Scanning for BLE devices. Search for a specific device on the left using arrows or j/k.",
            ), 
            Line::from(""),  
            Line::from(
                "To connect directly to a specific characteristic, use the following args: --device <NAME_SEARCH> and --characteristic <CHAR_SEARCH>.",
            ), 
            Line::from(""), 
            Line::from("You can provide names for your custom GATT services and characteristics by using the --names-map <FILE_PATH> argument if you are working with a specific service or device.")
        ])
        .wrap(Wrap { trim: true })
        .block(block);

        f.render_widget(paragraph, area);

        Ok(())
    }
}
