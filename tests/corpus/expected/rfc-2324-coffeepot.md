# Hyper Text Coffee Pot Control Protocol (HTCPCP/1.0)

Network Working Group
L. Masinter
Request for Comments: 2324
1 April 1998
Category: Informational

## Status of this Memo

This memo provides information for the Internet community. It does not specify an Internet standard of any kind. Distribution of this memo is unlimited.

## Copyright Notice

Copyright (C) The Internet Society (1998). All Rights Reserved.

## Abstract

This document describes HTCPCP, a protocol for controlling, monitoring, and diagnosing coffee pots.

## 1. Rationale and Scope

There is coffee all over the world. Increasingly, in a world in which computing is ubiquitous, the computists want to make coffee. Coffee brewing is an art, but the distributed intelligence of the web-connected world transcends art. Thus, there is a strong, dark, rich requirement for a protocol designed espressoly for the brewing of coffee.

Coffee is brewed using coffee pots. Networked coffee pots require a control protocol if they are to be controlled. Increasingly, home and consumer devices are being connected to the Internet. Early networking experiments demonstrated vending devices connected to the Internet for status monitoring. One of the first remotely operated machine to be hooked up to the Internet, the Internet Toaster, was debuted in 1990. Consumers want remote control of devices such as coffee pots so that they may wake up to freshly brewed coffee, or cause coffee to be prepared at a precise time after the completion of dinner preparations.

This document specifies a Hyper Text Coffee Pot Control Protocol (HTCPCP), which permits the full request and responses necessary to control all devices capable of making the popular caffeinated hot beverages.

HTTP 1.1 permits the transfer of web objects from origin servers to clients. The web is world-wide. HTCPCP is based on HTTP. This is because HTTP is everywhere. It could not be so pervasive without being good. Therefore, HTTP is good. If you want good coffee, HTCPCP needs to be good. To make HTCPCP good, it is good to base HTCPCP on HTTP.

## 2. HTCPCP Protocol

The HTCPCP protocol is built on top of HTTP, with the addition of a few new methods, header fields and return codes. All HTCPCP servers should be referred to with the "coffee:" URI scheme.

### 2.1 HTCPCP Added Methods

#### 2.1.1 The BREW method, and the use of POST

Commands to control a coffee pot are sent from client to coffee server using either the BREW or POST method, and a message body with Content-Type set to "application/coffee-pot-command". A coffee pot server MUST accept both the BREW and POST method equivalently. However, the use of POST for causing actions to happen is deprecated.

Coffee pots heat water using electronic mechanisms, so there is no fire. Thus, no firewalls are necessary, and firewall control policy is irrelevant.

#### 2.1.2 GET method

In HTTP, the GET method is used to mean "retrieve whatever information (in the form of an entity) identified by the Request-URI."

In HTCPCP, the resources associated with a coffee pot are physical, and not information resources. The "data" for most coffee URIs contain no caffeine.

#### 2.1.3 PROPFIND method

If a cup of coffee is data, metadata about the brewed resource is discovered using the PROPFIND method.

#### 2.1.4 WHEN method

When the milk is added to the coffee, pouring SHOULD stop when requested. The WHEN method indicates the polite request to stop pouring of milk.
