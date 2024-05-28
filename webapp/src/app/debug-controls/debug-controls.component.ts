import { Component } from '@angular/core';
import CoreSocket from '../models/communication/CoreSocket';
import { CoreSocketFactory } from '../models/communication/CoreSocketFactory';

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
    this.CoreSocket = CoreSocketFactory.create();
  }

  public send(data: string) {
    this.CoreSocket.send(data);
  }
}
