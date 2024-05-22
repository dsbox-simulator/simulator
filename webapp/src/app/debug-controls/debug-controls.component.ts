import { Component } from '@angular/core';
import CoreSocket from '../models/communication/CoreSocket';

@Component({
  selector: 'app-debug-controls',
  standalone: true,
  imports: [],
  templateUrl: './debug-controls.component.html',
  styleUrl: './debug-controls.component.css'
})
export class DebugControlsComponent {
  CoreSocket: CoreSocket;
  constructor() {
    this.CoreSocket = new CoreSocket();
  }

  public send(data: string) {
    this.CoreSocket.send(data);
  }
}
