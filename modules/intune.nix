# Microsoft Intune integration for NixOS
# Provides intune-portal and microsoft-identity-broker services
{ config, lib, pkgs, ... }:

{
  # Create the service user for the device broker
  users.users.microsoft-identity-broker = {
    isSystemUser = true;
    group = "microsoft-identity-broker";
    description = "Microsoft Identity Broker";
  };
  users.groups.microsoft-identity-broker = { };

  # Install packages
  environment.systemPackages = with pkgs; [
    intune-portal
    microsoft-identity-broker
    microsoft-edge           # Required for Intune compliance/auth
  ];

  # D-Bus configuration for the identity broker
  services.dbus.packages = [ pkgs.microsoft-identity-broker ];

  # System service for device broker
  systemd.services.microsoft-identity-device-broker = {
    description = "Microsoft Identity Device Broker Service";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "dbus";
      BusName = "com.microsoft.identity.devicebroker1";
      User = "microsoft-identity-broker";
      Group = "microsoft-identity-broker";
      RuntimeDirectory = "microsoft-identity-device-broker";
      StateDirectory = "microsoft-identity-device-broker";
      LogsDirectory = "microsoft-identity-device-broker";
      RuntimeDirectoryMode = "0700";
      StateDirectoryMode = "0700";
      LogsDirectoryMode = "0700";
      Environment = "JAVA_HOME=${pkgs.openjdk11}";
      ExecStart = "${pkgs.microsoft-identity-broker}/bin/microsoft-identity-device-broker";
      SuccessExitStatus = 143;
      TimeoutStopSec = 10;
      Restart = "on-failure";
      RestartSec = 5;
    };
  };

  # User service for identity broker (via Home Manager or systemd user units)
  systemd.user.services.microsoft-identity-broker = {
    description = "Microsoft Identity Broker Service";
    wantedBy = [ "default.target" ];
    serviceConfig = {
      Type = "dbus";
      BusName = "com.microsoft.identity.broker1";
      RuntimeDirectory = "microsoft-identity-broker";
      StateDirectory = "microsoft-identity-broker";
      LogsDirectory = "microsoft-identity-broker";
      RuntimeDirectoryMode = "0700";
      StateDirectoryMode = "0700";
      LogsDirectoryMode = "0700";
      Environment = "JAVA_HOME=${pkgs.openjdk11}";
      ExecStart = "${pkgs.microsoft-identity-broker}/bin/microsoft-identity-broker";
      SuccessExitStatus = 143;
      TimeoutStopSec = 10;
      Restart = "on-failure";
      RestartSec = 5;
      Slice = "background.slice";
    };
  };
}
