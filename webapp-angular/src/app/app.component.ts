import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { GraphComponent } from "./graph/graph.component";

@Component({
    selector: 'app-root',
    standalone: true,
    templateUrl: './app.component.html',
    styleUrl: './app.component.css',
    imports: [RouterOutlet, GraphComponent]
})

export class AppComponent {
  title = 'my-app';
  buttonText: string = "Click me";

  changeName(){
    this.buttonText = "Button Clicked"

  }
}