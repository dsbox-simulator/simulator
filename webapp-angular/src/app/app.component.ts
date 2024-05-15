import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { GraphComponent } from "./graph/graph.component";
import { DebugControlsComponent } from "./debug-controls/debug-controls.component";

@Component({
    selector: 'app-root',
    standalone: true,
    templateUrl: './app.component.html',
    styleUrl: './app.component.css',
    imports: [RouterOutlet, GraphComponent, DebugControlsComponent]
})

export class AppComponent {
  title = 'my-app';
  buttonText: string = "Click me";

  changeName(){
    this.buttonText = "Button Clicked"

  }
}