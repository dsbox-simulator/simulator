import { ComponentFixture, TestBed } from '@angular/core/testing';

import { JsonPredicateComponent } from './json-predicate.component';

describe('JsonPredicateComponent', () => {
  let component: JsonPredicateComponent;
  let fixture: ComponentFixture<JsonPredicateComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [JsonPredicateComponent]
    })
    .compileComponents();
    
    fixture = TestBed.createComponent(JsonPredicateComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
