import { Component,HostListener } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { GraphComponent } from "./graph/graph.component";
import { DebugControlsComponent } from "./debug-controls/debug-controls.component";
import { EventTableComponent } from "./event-table/event-table.component";
import { EventStore } from './models/EventStore';
import { CoreSocketFactory } from './models/communication/CoreSocketFactory';
import { GraphInTransitComponent } from "./graph-in-transit/graph-in-transit.component";
import { JsonRpcEvent } from './models/communication/RpcEvent';

@Component({
    selector: 'app-root',
    standalone: true,
    templateUrl: './app.component.html',
    styleUrl: './app.component.scss',
    imports: [RouterOutlet, GraphComponent, DebugControlsComponent, EventTableComponent, GraphInTransitComponent]
})

export class AppComponent {
openFile() {
  const inputElement = document.getElementById('loadFile');
  if (inputElement) {
    inputElement.click();
  }
}

onFileSelected($event: Event) {
  const file: File = ($event.target as HTMLInputElement).files![0];
  console.log("Selected file:", file);
  if (file) {
    this.readFile(file).then(contents => {
      console.log("File contents:", contents);
      //EventStore.loadEvents(contents);

      const events = JSON.parse(contents) as JsonRpcEvent[];
      CoreSocketFactory.load(events);
    }).catch(error => {
      console.error("Error reading file:", error);
    });
  }
  
}

readFile(file: File): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      resolve(reader.result as string);
    };
    reader.onerror = () => {
      reject(reader.error);
    };
    reader.readAsText(file);
  });
}

@HostListener('document:keydown', ['$event'])
  handleKeyboardEvent(event: KeyboardEvent) {
    if (event.key === 'F1', []) {
      event.preventDefault();

      const coreSocket = CoreSocketFactory.create();
      coreSocket.call('step', []);
    }
    if (event.key === 'F2') {
      event.preventDefault();

      const coreSocket = CoreSocketFactory.create();
      coreSocket.call('resume', []);
    }
  }


loadEvents() {  

}

saveEvents() {
  EventStore.saveEvents();
}

  activeTab: string = 'home';

  selectTab(tab: string) {
    this.activeTab = tab;
  }

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

  private darkThemeClass = 'dark-theme';
  private lightThemeClass = 'light-theme';

  constructor() { }

  setLightTheme() {
    document.body.classList.remove(this.darkThemeClass);
    document.body.classList.add(this.lightThemeClass);
  }

  setDarkTheme() {
    document.body.classList.remove(this.lightThemeClass);
    document.body.classList.add(this.darkThemeClass);
  }

  toggleTheme(isDarkMode: boolean) {
    if (isDarkMode) {
      this.setDarkTheme();
    } else {
      this.setLightTheme();
    }
  }
}