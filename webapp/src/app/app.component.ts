import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { GraphComponent } from "./graph/graph.component";
import { DebugControlsComponent } from "./debug-controls/debug-controls.component";
import { EventTableComponent } from "./event-table/event-table.component";

@Component({
    selector: 'app-root',
    standalone: true,
    templateUrl: './app.component.html',
    styleUrl: './app.component.scss',
    imports: [RouterOutlet, GraphComponent, DebugControlsComponent, EventTableComponent]
})

export class AppComponent {

setDarkMode() {
  document.body.classList.add('dark-mode');
}
setLightMode() {
  document.body.classList.remove('dark-mode');
}
  title = 'my-app';
  buttonText: string = "Click me";

  changeName(){
    this.buttonText = "Button Clicked"

  }
}