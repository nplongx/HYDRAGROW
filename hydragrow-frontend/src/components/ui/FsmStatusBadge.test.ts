import { describe, it, expect } from 'vitest';
import { extractFaultCode } from './FsmStatusBadge';

describe('extractFaultCode', () => {
  it('should return null if state is undefined', () => {
    expect(extractFaultCode()).toBeNull();
  });

  it('should return null if state is empty', () => {
    expect(extractFaultCode('')).toBeNull();
  });

  it('should extract fault code from SystemFault prefix', () => {
    expect(extractFaultCode('SystemFault: Error1')).toBe('Error1');
    expect(extractFaultCode('SystemFault:Error2')).toBe('Error2');
  });

  it('should extract fault code from Fault prefix', () => {
    expect(extractFaultCode('Fault: Error1')).toBe('Error1');
    expect(extractFaultCode('Fault:Error2')).toBe('Error2');
  });

  it('should handle leading and trailing spaces correctly', () => {
    expect(extractFaultCode('SystemFault:   Error3   ')).toBe('Error3');
    expect(extractFaultCode('Fault:   Error4   ')).toBe('Error4');
  });

  it('should extract fault code from valid JSON string with Fault key', () => {
    expect(extractFaultCode('{"Fault": "PhDosingFailed"}')).toBe('PhDosingFailed');
    expect(extractFaultCode('{"Fault":"TankEmpty"}')).toBe('TankEmpty');
  });

  it('should handle numbers in JSON Fault key and convert them to string', () => {
    expect(extractFaultCode('{"Fault": 404}')).toBe('404');
  });

  it('should return null for valid JSON string without Fault key', () => {
    expect(extractFaultCode('{"State": "Ok"}')).toBeNull();
    expect(extractFaultCode('{"status": "error"}')).toBeNull();
  });

  it('should return null for invalid JSON string starting with {', () => {
    expect(extractFaultCode('{InvalidJson:')).toBeNull();
    expect(extractFaultCode('{"Fault": "Error", missing closing bracket')).toBeNull();
  });

  it('should return null for states that do not match any fault pattern', () => {
    expect(extractFaultCode('Monitoring')).toBeNull();
    expect(extractFaultCode('DosingCycleComplete')).toBeNull();
    expect(extractFaultCode('SystemBooting')).toBeNull();
  });
});
